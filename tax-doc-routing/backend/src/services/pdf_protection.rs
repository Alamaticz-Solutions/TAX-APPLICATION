//! Password-protects the client copies of a PDF (Pega's `MoveFileToLTFolder_EP`, the "encrypted
//! path"). The password is the client's Read Write Password from the client record. In memory
//! only: nothing is written to disk, and a failure never falls back to an unprotected copy.

use std::{collections::BTreeMap, sync::Arc};

use lopdf::{
    encryption::{crypt_filters::{Aes128CryptFilter, CryptFilter}, EncryptionState, EncryptionVersion, Permissions},
    Document, Object,
};

use crate::services::document_store::workflow::PdfProtector;

/// Encrypts with the PDF standard security handler (AES-128, PDF 1.5), opened by `password`.
pub struct ClientPasswordProtector {
    password: String,
}

impl ClientPasswordProtector {
    pub fn new(password: &str) -> Self {
        Self { password: password.to_string() }
    }
}

impl PdfProtector for ClientPasswordProtector {
    fn protect(&self, pdf: &[u8]) -> Result<Vec<u8>, String> {
        if self.password.trim().is_empty() {
            return Err("this client has no Read Write Password to protect the PDF with".into());
        }
        let mut document = Document::load_mem(pdf).map_err(|e| format!("the file is not a readable PDF: {e}"))?;
        if document.is_encrypted() {
            return Err("the PDF is already password protected".into());
        }
        // The key is derived from the file's /ID entry. The PDF spec makes /ID optional for an
        // unencrypted file, so add one when the producer left it out.
        if document.trailer.get(b"ID").is_err() {
            let id = uuid::Uuid::new_v4().as_bytes().to_vec();
            document.trailer.set("ID", Object::Array(vec![Object::string_literal(id.clone()), Object::string_literal(id)]));
        }
        let crypt_filter: Arc<dyn CryptFilter> = Arc::new(Aes128CryptFilter);
        let version = EncryptionVersion::V4 {
            document: &document,
            encrypt_metadata: true,
            crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), crypt_filter)]),
            stream_filter: b"StdCF".to_vec(),
            string_filter: b"StdCF".to_vec(),
            owner_password: &self.password,
            user_password: &self.password,
            permissions: Permissions::all(),
        };
        let state = EncryptionState::try_from(version).map_err(|e| format!("could not set up PDF encryption: {e}"))?;
        document.encrypt(&state).map_err(|e| format!("could not encrypt the PDF: {e}"))?;
        let mut out = Vec::new();
        document.save_to(&mut out).map_err(|e| format!("could not write the protected PDF: {e}"))?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use lopdf::{dictionary, Stream};

    use super::*;

    fn sample_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(dictionary! {}, b"BT ET".to_vec()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn the_protected_copy_needs_the_client_password() {
        let original = sample_pdf();
        let protected = ClientPasswordProtector::new("Kpds-client-pw").protect(&original).unwrap();
        assert_ne!(protected, original);

        let mut opened = Document::load_mem(&protected).unwrap();
        assert!(opened.is_encrypted());
        assert!(opened.decrypt("wrong-password").is_err(), "a wrong password must not open it");

        let mut opened = Document::load_mem(&protected).unwrap();
        assert!(opened.decrypt("Kpds-client-pw").is_ok(), "the client password opens it");
    }

    #[test]
    fn a_pdf_without_a_file_id_can_still_be_protected() {
        let original = sample_pdf();
        assert!(Document::load_mem(&original).unwrap().trailer.get(b"ID").is_err(), "the sample has no /ID");
        let protected = ClientPasswordProtector::new("pw").protect(&original).unwrap();
        assert!(Document::load_mem(&protected).unwrap().decrypt("pw").is_ok());
    }

    #[test]
    fn it_fails_closed_without_a_password_or_with_a_bad_file() {
        assert!(ClientPasswordProtector::new("   ").protect(&sample_pdf()).is_err());
        assert!(ClientPasswordProtector::new("pw").protect(b"this is not a pdf").is_err());
    }

    #[test]
    fn an_already_protected_pdf_is_refused_rather_than_double_encrypted() {
        let once = ClientPasswordProtector::new("pw").protect(&sample_pdf()).unwrap();
        assert!(ClientPasswordProtector::new("pw").protect(&once).is_err());
    }
}
