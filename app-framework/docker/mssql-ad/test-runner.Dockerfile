# ODBC e2e test runner — Microsoft ODBC Driver 18 (sql_password) + FreeTDS (ntlm).
FROM rust:1.95-bookworm

USER root
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates curl gnupg unixodbc unixodbc-dev \
        tdsodbc freetds-bin freetds-common \
    && curl -fsSL https://packages.microsoft.com/keys/microsoft.asc \
        | gpg --dearmor -o /usr/share/keyrings/microsoft-prod.gpg \
    && echo "deb [arch=amd64 signed-by=/usr/share/keyrings/microsoft-prod.gpg] https://packages.microsoft.com/debian/12/prod bookworm main" \
        > /etc/apt/sources.list.d/mssql-release.list \
    && apt-get update \
    && ACCEPT_EULA=Y apt-get install -y --no-install-recommends msodbcsql18 \
    && rm -rf /var/lib/apt/lists/* \
    && MS_DRIVER_PATH="$(ls /opt/microsoft/msodbcsql18/lib64/libmsodbcsql-18*.so.* 2>/dev/null | head -1)" \
    && printf '%s\n' \
        '[FreeTDS]' \
        'Description=FreeTDS Driver' \
        'Driver=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so' \
        'Setup=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so' \
        'UsageCount=1' \
        '' \
        '[ODBC Driver 18 for SQL Server]' \
        "Description=Microsoft ODBC Driver 18 for SQL Server" \
        "Driver=${MS_DRIVER_PATH}" \
        'UsageCount=1' \
        > /etc/odbcinst.ini

ENV PATH=/usr/local/cargo/bin:/usr/local/rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin:$PATH \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup

WORKDIR /src
CMD ["bash", "-lc", "cargo test -p appfw-provider-mssql --test ntlm_e2e --test sql_password_odbc_e2e -- --ignored --nocapture"]
