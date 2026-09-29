use std::{borrow::Cow, env, ffi::CString, ptr, sync::OnceLock};

use appfw_runtime::RuntimeError;
use odbc_api::{
    handles::{
        DiagnosticStream, Diagnostics, Environment as OdbcEnvironment, SqlResult, SqlText,
        Statement, StatementImpl,
    },
    parameter::VarCharArray,
    sys::{
        AttrOdbcVersion, CDataType, ConnectionAttribute, Len, ParamType, Pointer, SQLBindParameter,
        SQLSetConnectAttr, SqlDataType, SqlReturn, IS_POINTER, IS_UINTEGER, NULL_DATA,
    },
};

use crate::param::SqlParam;
use crate::row::MssqlRow;

pub const MS_ODBC_DRIVER_18: &str = "ODBC Driver 18 for SQL Server";
pub const FREETDS_DRIVER: &str = "FreeTDS";
pub const DRIVER_NAME_ENV: &str = "APP_MSSQL_ODBC_DRIVER";
pub const FABRIC_DRIVER_NAME_ENV: &str = "APP_FABRIC_ODBC_DRIVER";

// SQL Server ODBC driver-specific pre-connect attribute for ACCESSTOKEN.
pub const SQL_COPT_SS_ACCESS_TOKEN: ConnectionAttribute = ConnectionAttribute(1256);

pub const LOGIN_TIMEOUT_ENV: &str = "APP_MSSQL_LOGIN_TIMEOUT_SECS";
pub const DEFAULT_LOGIN_TIMEOUT_SECS: u32 = 30;

pub fn login_timeout_secs() -> u32 {
    env::var(LOGIN_TIMEOUT_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(DEFAULT_LOGIN_TIMEOUT_SECS)
}

pub fn resolve_odbc_driver_name() -> String {
    env::var(DRIVER_NAME_ENV)
        .or_else(|_| env::var(FABRIC_DRIVER_NAME_ENV))
        .unwrap_or_else(|_| MS_ODBC_DRIVER_18.to_string())
}

pub fn odbc_environment() -> Result<&'static OdbcEnvironment, RuntimeError> {
    static ENV: OnceLock<Result<SharedOdbcEnvironment, String>> = OnceLock::new();
    match ENV.get_or_init(init_odbc_environment) {
        Ok(env) => Ok(&env.0),
        Err(err) => Err(RuntimeError::DataAccess(err.clone())),
    }
}

pub fn connect_with_connection_string(
    connection_string: &str,
) -> Result<OdbcConnection, RuntimeError> {
    if connection_string.as_bytes().contains(&0) {
        return Err(RuntimeError::DataAccess(
            "ODBC connection string contains NUL byte".to_string(),
        ));
    }

    let env = odbc_environment()?;
    let mut dbc = sql_result(env.allocate_connection(), env, "allocate ODBC connection")?;
    set_login_timeout(&mut dbc)?;
    sql_result(
        dbc.connect_with_connection_string(&SqlText::new(connection_string)),
        &dbc,
        "connect with ODBC connection string",
    )?;

    Ok(OdbcConnection {
        dbc,
        _access_token_attr: None,
    })
}

pub fn connect_with_access_token(
    connection_string: &str,
    access_token: &str,
) -> Result<OdbcConnection, RuntimeError> {
    if connection_string.as_bytes().contains(&0) {
        return Err(RuntimeError::DataAccess(
            "ODBC connection string contains NUL byte".to_string(),
        ));
    }

    let env = odbc_environment()?;
    let mut dbc = sql_result(env.allocate_connection(), env, "allocate ODBC connection")?;

    let mut access_token_attr = access_token_attribute(access_token);
    sql_return(
        unsafe {
            SQLSetConnectAttr(
                dbc.as_sys(),
                SQL_COPT_SS_ACCESS_TOKEN,
                access_token_attr.as_mut_ptr() as Pointer,
                IS_POINTER,
            )
        },
        &dbc,
        "set Entra access token",
    )?;

    set_login_timeout(&mut dbc)?;
    sql_result(
        dbc.connect_with_connection_string(&SqlText::new(connection_string)),
        &dbc,
        "connect with Entra access token",
    )?;

    Ok(OdbcConnection {
        dbc,
        _access_token_attr: Some(access_token_attr),
    })
}

pub fn access_token_attribute(access_token: &str) -> Vec<u8> {
    let token_bytes = access_token
        .as_bytes()
        .iter()
        .flat_map(|byte| [*byte, 0])
        .collect::<Vec<_>>();
    let mut attr = Vec::with_capacity(4 + token_bytes.len());
    attr.extend_from_slice(&(token_bytes.len() as u32).to_le_bytes());
    attr.extend_from_slice(&token_bytes);
    attr
}

fn set_login_timeout(dbc: &mut odbc_api::handles::Connection<'static>) -> Result<(), RuntimeError> {
    let timeout = login_timeout_secs();
    sql_return(
        unsafe {
            SQLSetConnectAttr(
                dbc.as_sys(),
                ConnectionAttribute::LOGIN_TIMEOUT,
                timeout as usize as Pointer,
                IS_UINTEGER,
            )
        },
        dbc,
        "set ODBC login timeout",
    )?;
    Ok(())
}

pub fn odbc_sql_and_params(
    sql: String,
    params: Vec<SqlParam>,
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    if params.is_empty() {
        return Ok((sql, params));
    }

    let bytes = sql.as_bytes();
    let mut rewritten_sql = String::with_capacity(sql.len());
    let mut rewritten_params = Vec::new();
    let mut used = vec![false; params.len()];
    let mut last = 0;
    let mut cursor = 0;

    while cursor < bytes.len() {
        if bytes[cursor] == b'@'
            && cursor + 2 < bytes.len()
            && bytes[cursor + 1] == b'P'
            && bytes[cursor + 2].is_ascii_digit()
        {
            let mut end = cursor + 2;
            let mut index = 0usize;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                index = index
                    .saturating_mul(10)
                    .saturating_add((bytes[end] - b'0') as usize);
                end += 1;
            }

            if index == 0 || index > params.len() {
                return Err(RuntimeError::DataAccess(format!(
                    "ODBC SQL references parameter @P{index}, but {} parameter(s) were supplied",
                    params.len()
                )));
            }

            rewritten_sql.push_str(&sql[last..cursor]);
            rewritten_sql.push('?');
            rewritten_params.push(params[index - 1].clone());
            used[index - 1] = true;
            cursor = end;
            last = end;
            continue;
        }

        cursor += 1;
    }

    if rewritten_params.is_empty() {
        return Ok((sql, params));
    }

    if let Some(unused_index) = used.iter().position(|used| !*used).map(|index| index + 1) {
        return Err(RuntimeError::DataAccess(format!(
            "ODBC SQL did not reference supplied parameter @P{unused_index}"
        )));
    }

    rewritten_sql.push_str(&sql[last..]);
    Ok((rewritten_sql, rewritten_params))
}

pub struct OdbcConnection {
    dbc: odbc_api::handles::Connection<'static>,
    _access_token_attr: Option<Vec<u8>>,
}

impl OdbcConnection {
    pub fn prepare(&mut self, sql: &str) -> Result<OdbcStatement<'_>, RuntimeError> {
        if sql.as_bytes().contains(&0) {
            return Err(RuntimeError::DataAccess(
                "ODBC SQL text contains NUL byte".to_string(),
            ));
        }

        let mut stmt = sql_result(
            self.dbc.allocate_statement(),
            &self.dbc,
            "allocate ODBC statement",
        )?;
        sql_result(
            stmt.prepare(&SqlText::new(sql)),
            &stmt,
            "prepare ODBC statement",
        )?;
        Ok(OdbcStatement {
            stmt,
            bindings: Vec::new(),
        })
    }

    /// Run a statement without the prepare/execute RPC path (uses SQL batch on FreeTDS).
    pub fn execute_direct(&mut self, sql: &str) -> Result<OdbcStatement<'_>, RuntimeError> {
        if sql.as_bytes().contains(&0) {
            return Err(RuntimeError::DataAccess(
                "ODBC SQL text contains NUL byte".to_string(),
            ));
        }

        let mut stmt = sql_result(
            self.dbc.allocate_statement(),
            &self.dbc,
            "allocate ODBC statement",
        )?;
        sql_result(
            unsafe { stmt.exec_direct(&SqlText::new(sql)) },
            &stmt,
            "execute ODBC statement directly",
        )?;
        Ok(OdbcStatement {
            stmt,
            bindings: Vec::new(),
        })
    }
}

impl Drop for OdbcConnection {
    fn drop(&mut self) {
        let _ = self.dbc.disconnect();
    }
}

pub struct OdbcStatement<'a> {
    stmt: StatementImpl<'a>,
    bindings: Vec<BoundParam>,
}

impl OdbcStatement<'_> {
    pub fn bind_params(&mut self, params: Vec<SqlParam>) -> Result<(), RuntimeError> {
        self.bindings = params
            .into_iter()
            .map(BoundParam::try_from_sql_param)
            .collect::<Result<Vec<_>, _>>()?;

        for (index, param) in self.bindings.iter_mut().enumerate() {
            let bind = param.binding();
            sql_return(
                unsafe {
                    SQLBindParameter(
                        self.stmt.as_sys(),
                        (index + 1) as u16,
                        ParamType::Input,
                        bind.c_type,
                        bind.sql_type,
                        bind.column_size,
                        0,
                        bind.value_ptr,
                        bind.buffer_len,
                        bind.indicator_ptr,
                    )
                },
                &self.stmt,
                "bind ODBC parameter",
            )?;
        }
        Ok(())
    }

    pub fn execute(&mut self) -> Result<(), RuntimeError> {
        sql_result(
            unsafe { self.stmt.execute() },
            &self.stmt,
            "execute ODBC statement",
        )
    }

    pub fn rows_affected(&mut self) -> Result<u64, RuntimeError> {
        let count = sql_result(self.stmt.row_count(), &self.stmt, "read ODBC row count")?;
        Ok(count.max(0) as u64)
    }

    pub fn fetch(&mut self) -> Result<bool, RuntimeError> {
        match unsafe { self.stmt.fetch() } {
            SqlResult::Success(()) | SqlResult::SuccessWithInfo(()) => Ok(true),
            SqlResult::NoData => Ok(false),
            other => sql_result(other, &self.stmt, "fetch ODBC row").map(|_| true),
        }
    }

    /// Advance to the next result set in a multi-statement batch.
    /// Returns `false` when there are no more result sets (`SQL_NO_DATA`).
    pub fn more_results(&mut self) -> Result<bool, RuntimeError> {
        match unsafe { self.stmt.more_results() } {
            SqlResult::Success(()) | SqlResult::SuccessWithInfo(()) => Ok(true),
            SqlResult::NoData => Ok(false),
            other => sql_result(other, &self.stmt, "advance ODBC result set").map(|_| true),
        }
    }

    pub fn get_string(&mut self, col: u16) -> Result<Option<String>, RuntimeError> {
        let mut out = Vec::<u8>::new();
        loop {
            let mut buffer = VarCharArray::<8192>::NULL;
            match self.stmt.get_data(col, &mut buffer) {
                SqlResult::Success(()) | SqlResult::SuccessWithInfo(()) => {}
                SqlResult::NoData => break,
                other => {
                    sql_result(other, &self.stmt, "read ODBC column")?;
                }
            }

            let Some(bytes) = buffer.as_bytes() else {
                return Ok(None);
            };
            out.extend_from_slice(bytes);
            if buffer.is_complete() {
                break;
            }
        }
        String::from_utf8(out)
            .map(Some)
            .map_err(|err| RuntimeError::DataAccess(format!("ODBC returned invalid UTF-8: {err}")))
    }

    pub fn column_names(&mut self) -> Result<Vec<String>, RuntimeError> {
        let num_cols = sql_result(
            self.stmt.num_result_cols(),
            &self.stmt,
            "count ODBC result columns",
        )?;
        let mut names = Vec::with_capacity(num_cols as usize);
        for col in 1..=num_cols {
            let mut buf = vec![0; 1024];
            sql_result(
                self.stmt.col_name(col as u16, &mut buf),
                &self.stmt,
                "read ODBC column name",
            )?;
            let name = odbc_api::handles::slice_to_utf8(&buf).map_err(|err| {
                RuntimeError::DataAccess(format!("ODBC column name is not valid UTF-8: {err}"))
            })?;
            names.push(name);
        }
        Ok(names)
    }
}

pub fn fetch_rows_as_mssql_rows(
    stmt: &mut OdbcStatement<'_>,
) -> Result<Vec<MssqlRow>, RuntimeError> {
    // SQL Server batches (BEGIN/IF/INSERT/SELECT) often surface DONE_IN_PROC
    // row-count "results" with zero columns before the real SELECT. Fetching
    // those yields SQLSTATE 24000 Invalid cursor state.
    let column_names = loop {
        let names = stmt.column_names()?;
        if !names.is_empty() {
            break names;
        }
        if !stmt.more_results()? {
            return Ok(Vec::new());
        }
    };
    let mut rows = Vec::new();

    while stmt.fetch()? {
        let mut row = MssqlRow::default();
        for (index, name) in column_names.iter().enumerate() {
            let value = stmt.get_string((index + 1) as u16)?;
            row.insert(name.clone(), value);
        }
        rows.push(row);
    }

    Ok(rows)
}

pub fn sql_result<T>(
    result: SqlResult<T>,
    diagnostics_handle: &(impl Diagnostics + ?Sized),
    action: &str,
) -> Result<T, RuntimeError> {
    match result {
        SqlResult::Success(value) | SqlResult::SuccessWithInfo(value) => Ok(value),
        SqlResult::Error { function } => Err(RuntimeError::DataAccess(format!(
            "ODBC failed to {action} in {function}: {}",
            diagnostics(diagnostics_handle)
        ))),
        SqlResult::NoData => Err(RuntimeError::DataAccess(format!(
            "ODBC failed to {action}: no data; {}",
            diagnostics(diagnostics_handle)
        ))),
        SqlResult::NeedData => Err(RuntimeError::DataAccess(format!(
            "ODBC failed to {action}: more parameter data required; {}",
            diagnostics(diagnostics_handle)
        ))),
        SqlResult::StillExecuting => Err(RuntimeError::DataAccess(format!(
            "ODBC failed to {action}: operation still executing; {}",
            diagnostics(diagnostics_handle)
        ))),
    }
}

pub fn sql_return(
    ret: SqlReturn,
    diagnostics_handle: &(impl Diagnostics + ?Sized),
    action: &str,
) -> Result<(), RuntimeError> {
    match ret {
        SqlReturn::SUCCESS | SqlReturn::SUCCESS_WITH_INFO => Ok(()),
        other => Err(RuntimeError::DataAccess(format!(
            "ODBC failed to {action}: {other:?}; {}",
            diagnostics(diagnostics_handle)
        ))),
    }
}

pub fn diagnostics(handle: &(impl Diagnostics + ?Sized)) -> String {
    let mut messages = Vec::new();
    let mut stream = DiagnosticStream::new(handle);
    while let Some(record) = stream.next() {
        messages.push(sanitize_diagnostic_message(&record.to_string()));
        if messages.len() >= 5 {
            break;
        }
    }

    if messages.is_empty() {
        "no ODBC diagnostics returned".to_string()
    } else {
        messages.join("; ")
    }
}

pub fn sanitize_diagnostic_message(message: &str) -> String {
    if let Some(start) = message.find("Can't open lib") {
        if let Some(relative_end) = message[start..].find(" : ") {
            let end = start + relative_end;
            return format!(
                "{}: driver library not found in configured ODBC driver paths",
                &message[..end]
            );
        }
    }
    message.to_string()
}

struct OdbcParamBinding {
    c_type: CDataType,
    sql_type: SqlDataType,
    column_size: usize,
    value_ptr: Pointer,
    buffer_len: Len,
    indicator_ptr: *mut Len,
}

enum BoundParam {
    Null {
        indicator: Len,
        sql_type: SqlDataType,
    },
    Bit {
        value: u8,
        indicator: Len,
    },
    I16 {
        value: i16,
        indicator: Len,
    },
    I32 {
        value: i32,
        indicator: Len,
    },
    I64 {
        value: i64,
        indicator: Len,
    },
    F32 {
        value: f32,
        indicator: Len,
    },
    F64 {
        value: f64,
        indicator: Len,
    },
    Text {
        value: CString,
        indicator: Len,
    },
    Binary {
        value: Vec<u8>,
        indicator: Len,
    },
}

impl BoundParam {
    fn try_from_sql_param(param: SqlParam) -> Result<Self, RuntimeError> {
        match param {
            SqlParam::Bit(value) => nullable(value, SqlDataType::EXT_BIT, |value| {
                Ok(BoundParam::Bit {
                    value: u8::from(value),
                    indicator: 0,
                })
            }),
            SqlParam::I16(value) => nullable(value, SqlDataType::SMALLINT, |value| {
                Ok(BoundParam::I16 {
                    value,
                    indicator: 0,
                })
            }),
            SqlParam::I32(value) => nullable(value, SqlDataType::INTEGER, |value| {
                Ok(BoundParam::I32 {
                    value,
                    indicator: 0,
                })
            }),
            SqlParam::I64(value) => nullable(value, SqlDataType::EXT_BIG_INT, |value| {
                Ok(BoundParam::I64 {
                    value,
                    indicator: 0,
                })
            }),
            SqlParam::U8(value) => nullable(value.map(i16::from), SqlDataType::SMALLINT, |value| {
                Ok(BoundParam::I16 {
                    value,
                    indicator: 0,
                })
            }),
            SqlParam::F32(value) => nullable(value, SqlDataType::REAL, |value| {
                Ok(BoundParam::F32 {
                    value,
                    indicator: 0,
                })
            }),
            SqlParam::F64(value) => nullable(value, SqlDataType::DOUBLE, |value| {
                Ok(BoundParam::F64 {
                    value,
                    indicator: 0,
                })
            }),
            SqlParam::Guid(value) => nullable(
                value.map(|uuid| uuid.to_string()),
                SqlDataType::VARCHAR,
                |value| text_param(Cow::Owned(value)),
            ),
            SqlParam::String(value) => match value {
                Some(value) => text_param(Cow::Owned(value)),
                None => Ok(BoundParam::Null {
                    indicator: NULL_DATA,
                    sql_type: SqlDataType::VARCHAR,
                }),
            },
            SqlParam::Binary(value) => nullable(value, SqlDataType::EXT_VAR_BINARY, |value| {
                let indicator = value.len() as Len;
                Ok(BoundParam::Binary { value, indicator })
            }),
        }
    }

    fn binding(&mut self) -> OdbcParamBinding {
        match self {
            BoundParam::Null {
                indicator,
                sql_type,
            } => OdbcParamBinding {
                c_type: CDataType::Char,
                sql_type: *sql_type,
                column_size: 0,
                value_ptr: ptr::null_mut(),
                buffer_len: 0,
                indicator_ptr: indicator,
            },
            BoundParam::Bit { value, indicator } => OdbcParamBinding {
                c_type: CDataType::Bit,
                sql_type: SqlDataType::EXT_BIT,
                column_size: 1,
                value_ptr: value as *mut u8 as Pointer,
                buffer_len: 0,
                indicator_ptr: indicator,
            },
            BoundParam::I16 { value, indicator } => OdbcParamBinding {
                c_type: CDataType::SShort,
                sql_type: SqlDataType::SMALLINT,
                column_size: 0,
                value_ptr: value as *mut i16 as Pointer,
                buffer_len: 0,
                indicator_ptr: indicator,
            },
            BoundParam::I32 { value, indicator } => OdbcParamBinding {
                c_type: CDataType::SLong,
                sql_type: SqlDataType::INTEGER,
                column_size: 0,
                value_ptr: value as *mut i32 as Pointer,
                buffer_len: 0,
                indicator_ptr: indicator,
            },
            BoundParam::I64 { value, indicator } => OdbcParamBinding {
                c_type: CDataType::SBigInt,
                sql_type: SqlDataType::EXT_BIG_INT,
                column_size: 0,
                value_ptr: value as *mut i64 as Pointer,
                buffer_len: 0,
                indicator_ptr: indicator,
            },
            BoundParam::F32 { value, indicator } => OdbcParamBinding {
                c_type: CDataType::Float,
                sql_type: SqlDataType::REAL,
                column_size: 0,
                value_ptr: value as *mut f32 as Pointer,
                buffer_len: 0,
                indicator_ptr: indicator,
            },
            BoundParam::F64 { value, indicator } => OdbcParamBinding {
                c_type: CDataType::Double,
                sql_type: SqlDataType::DOUBLE,
                column_size: 0,
                value_ptr: value as *mut f64 as Pointer,
                buffer_len: 0,
                indicator_ptr: indicator,
            },
            BoundParam::Text { value, indicator } => OdbcParamBinding {
                c_type: CDataType::Char,
                sql_type: SqlDataType::VARCHAR,
                column_size: value.as_bytes().len(),
                value_ptr: value.as_ptr() as Pointer,
                buffer_len: value.as_bytes().len() as Len,
                indicator_ptr: indicator,
            },
            BoundParam::Binary { value, indicator } => OdbcParamBinding {
                c_type: CDataType::Binary,
                sql_type: SqlDataType::EXT_VAR_BINARY,
                column_size: value.len(),
                value_ptr: value.as_mut_ptr() as Pointer,
                buffer_len: value.len() as Len,
                indicator_ptr: indicator,
            },
        }
    }
}

fn nullable<T>(
    value: Option<T>,
    sql_type: SqlDataType,
    f: impl FnOnce(T) -> Result<BoundParam, RuntimeError>,
) -> Result<BoundParam, RuntimeError> {
    Ok(match value {
        Some(value) => f(value)?,
        None => BoundParam::Null {
            indicator: NULL_DATA,
            sql_type,
        },
    })
}

fn text_param(value: Cow<'static, str>) -> Result<BoundParam, RuntimeError> {
    let bytes = value.as_bytes();
    let cstring = CString::new(bytes).map_err(|_| {
        RuntimeError::DataAccess("ODBC string parameter contains NUL byte".to_string())
    })?;
    Ok(BoundParam::Text {
        indicator: cstring.as_bytes().len() as Len,
        value: cstring,
    })
}

struct SharedOdbcEnvironment(OdbcEnvironment);

// We only use the global environment to allocate connections and never enumerate mutable
// driver/data-source state through it.
unsafe impl Sync for SharedOdbcEnvironment {}

fn init_odbc_environment() -> Result<SharedOdbcEnvironment, String> {
    let env = match OdbcEnvironment::new() {
        SqlResult::Success(env) | SqlResult::SuccessWithInfo(env) => env,
        SqlResult::Error { function } => {
            return Err(format!("ODBC failed to allocate environment in {function}"));
        }
        other => {
            return Err(format!("ODBC failed to allocate environment: {other:?}"));
        }
    };

    match env.declare_version(AttrOdbcVersion::Odbc3_80) {
        SqlResult::Success(()) | SqlResult::SuccessWithInfo(()) => Ok(SharedOdbcEnvironment(env)),
        SqlResult::Error { function } => Err(format!(
            "ODBC failed to configure ODBC environment in {function}: {}",
            diagnostics(&env)
        )),
        other => Err(format!(
            "ODBC failed to configure ODBC environment: {other:?}; {}",
            diagnostics(&env)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_token_attribute_is_utf16le_with_length_prefix() {
        let attr = access_token_attribute("abc");
        assert_eq!(&attr[..4], &(6u32).to_le_bytes());
        assert_eq!(&attr[4..], &[b'a', 0, b'b', 0, b'c', 0]);
    }

    #[test]
    fn missing_driver_diagnostic_drops_local_search_paths() {
        let message = "State: 01000, Native error: 0, Message: [unixODBC][Driver Manager]Can't open lib 'ODBC Driver 18 for SQL Server' : dlopen(ODBC Driver 18 for SQL Server, 0x0009): tried: '/private/path'";
        let sanitized = sanitize_diagnostic_message(message);
        assert_eq!(
            sanitized,
            "State: 01000, Native error: 0, Message: [unixODBC][Driver Manager]Can't open lib 'ODBC Driver 18 for SQL Server': driver library not found in configured ODBC driver paths"
        );
        assert!(!sanitized.contains("/private/path"));
    }

    #[test]
    fn odbc_sql_rewrites_named_params_to_positional_markers() {
        let params = vec![
            SqlParam::I32(Some(1)),
            SqlParam::I32(Some(2)),
            SqlParam::I32(Some(3)),
            SqlParam::I32(Some(4)),
            SqlParam::I32(Some(5)),
            SqlParam::I32(Some(6)),
            SqlParam::I32(Some(7)),
            SqlParam::I32(Some(8)),
            SqlParam::I32(Some(9)),
            SqlParam::I32(Some(10)),
        ];
        let (sql, params) = odbc_sql_and_params(
            "SELECT @P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9, @P10".to_string(),
            params,
        )
        .expect("rewrite");

        assert_eq!(sql, "SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?");
        assert_eq!(params.len(), 10);
    }

    #[test]
    fn odbc_sql_duplicates_repeated_named_params_in_occurrence_order() {
        let params = vec![SqlParam::I32(Some(42))];
        let (sql, params) =
            odbc_sql_and_params("SELECT @P1 WHERE @P1 > 0".to_string(), params).expect("rewrite");

        assert_eq!(sql, "SELECT ? WHERE ? > 0");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn odbc_sql_rejects_unreferenced_params() {
        let params = vec![SqlParam::I32(Some(1)), SqlParam::I32(Some(2))];
        let err = odbc_sql_and_params("SELECT @P2".to_string(), params);
        assert!(err.is_err());
    }
}
