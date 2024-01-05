use std::ops::Range;
use std::string::ToString;
use alvarium_annotator::{Annotation, Annotator, constants, derive_hash, serialise_and_sign};
use alvarium_annotator::constants::AnnotationType;
use alvarium_sdk_rust::{config, factories::new_hash_provider, providers::sign_provider::SignatureProviderWrap};
use alvarium_sdk_rust::config::Signable;
use alvarium_sdk_rust::factories::new_signature_provider;
use crate::mock_sensor::SensorReading;
use crate::errors::Result;


lazy_static! {
    /// Annotation Type definition
    pub static ref ANNOTATION_THRESHOLD: AnnotationType = AnnotationType("threshold".to_string());
}

/// Defines a new annotator type that will implement the Annotator trait
pub struct ThresholdAnnotator {
    /// Hashing algorithm used for checksums
    hash: constants::HashType,
    /// Type of annotation (a wrapper around a string definition)
    kind: AnnotationType,
    /// Signature provider for signing data
    sign: SignatureProviderWrap,
    /// Threshold limits for custom annotation
    range: Range<u8>,
}

impl ThresholdAnnotator {
    pub fn new(cfg: &config::SdkInfo, range: Range<u8>) -> Result<impl Annotator<Error = alvarium_sdk_rust::errors::Error>> {
        Ok(ThresholdAnnotator {
            hash: cfg.hash.hash_type.clone(),
            kind: ANNOTATION_THRESHOLD.clone(),
            sign: new_signature_provider(&cfg.signature)?,
            range,
        })
    }

}

/// Implementation of the annotate() function for generating a threshold Annotation
impl Annotator for ThresholdAnnotator {
    type Error = alvarium_sdk_rust::errors::Error;
    fn annotate(&mut self, data: &[u8]) -> alvarium_sdk_rust::errors::Result<Annotation> {
        let hasher = new_hash_provider(&self.hash)?;
        let signable: std::result::Result<Signable, serde_json::Error> = serde_json::from_slice(data);
        let key = match signable {
            Ok(signable) => derive_hash(hasher, signable.seed.as_bytes()),
            Err(_) => derive_hash(hasher, data),
        };
        match gethostname::gethostname().to_str() {
            Some(host) => {
                // For the sake of this example we will use both a signable and non signable reading
                // So we will need to do a match to check which one
                let reading: std::result::Result<SensorReading, serde_json::Error> = serde_json::from_slice(data);
                let within_threshold = match reading {
                    Ok(reading) => reading.value <= self.range.end && reading.value >= self.range.start,
                    Err(_) => {
                        let signable: std::result::Result<Signable, serde_json::Error> = serde_json::from_slice(data);
                        match signable {
                            Ok(signable) => {
                                let reading: SensorReading = serde_json::from_str(&signable.seed).unwrap();
                                reading.value <= self.range.end && reading.value >= self.range.start
                            }
                            Err(_) => false
                        }
                    }
                };

                let mut annotation = Annotation::new(&key, self.hash.clone(), host, self.kind.clone(), within_threshold);
                let signature = serialise_and_sign(&self.sign, &annotation)?;
                annotation.with_signature(&signature);
                Ok(annotation)
            },
            None => {
                Err(alvarium_sdk_rust::errors::Error::NoHostName.into())
            }
        }
    }
}
