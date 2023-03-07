use std::f64::NAN;

use distributed_id_allocator::{
    compressor::{ErrorEnum, IdCompressor as IdCompressorCore, IdRange},
    id_types::{LocalId, OpSpaceId, SessionId, SessionSpaceId, StableId},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct IdCompressor {
    compressor: IdCompressorCore,
    error_string: Option<String>,
}

const MAX_SAFE_INTEGER: i64 = (2 as i64).pow(53) - 1;

#[wasm_bindgen]
impl IdCompressor {
    #[wasm_bindgen(constructor)]
    pub fn new(session_id_string: String) -> Option<IdCompressor> {
        let session_id = match SessionId::from_uuid_string(&session_id_string) {
            Ok(id) => id,
            Err(_) => return None,
        };
        Some(IdCompressor {
            compressor: IdCompressorCore::new_with_session_id(session_id),
            error_string: None,
        })
    }

    pub fn set_cluster_capacity(&mut self, new_cluster_capacity: f64) -> Result<(), JsError> {
        if new_cluster_capacity.fract() != 0.0 {
            return Err(JsError::new("Non-integer cluster size."));
        }
        self.compressor
            .set_cluster_capacity(new_cluster_capacity as u64)
            .map_err(|err| JsError::new(err.get_error_string()))
    }

    pub fn generate_next_id(&mut self) -> f64 {
        let next_id = self.compressor.generate_next_id().id();
        debug_assert!(next_id >= -MAX_SAFE_INTEGER && next_id <= MAX_SAFE_INTEGER);
        next_id as f64
    }

    pub fn get_token(&mut self, uuid_string: String) -> Result<f64, JsError> {
        let session_id = match SessionId::from_uuid_string(&uuid_string) {
            Err(e) => {
                return Err(JsError::new(e.get_error_string()));
            }
            Ok(session_id) => session_id,
        };
        match self
            .compressor
            .get_session_token_from_session_id(session_id)
        {
            Err(e) => Err(JsError::new(e.get_error_string())),
            Ok(token) => Ok(token as f64),
        }
    }

    pub fn take_next_range(&mut self) -> InteropIdRange {
        let token = self.compressor.get_local_session_token() as f64;
        match self.compressor.take_next_range().range {
            Some((local, count)) => InteropIdRange {
                token,
                local: local.id() as f64,
                count: count as f64,
            },
            None => InteropIdRange {
                token,
                local: NAN,
                count: NAN,
            },
        }
    }

    pub fn finalize_range(
        &mut self,
        session_token: f64,
        range_base_local: f64,
        range_len: f64,
    ) -> Result<(), JsError> {
        let id = match self
            .compressor
            .get_session_id_from_session_token(session_token as usize)
        {
            Err(e) => {
                return Err(JsError::new(e.get_error_string()));
            }
            Ok(session_id) => session_id,
        };
        self.compressor
            .finalize_range(&IdRange {
                id,
                range: Some((LocalId::new(range_base_local as i64), range_len as u64)),
            })
            .map_err(|e| JsError::new(e.get_error_string()))
    }

    pub fn normalize_to_op_space(&mut self, session_space_id: f64) -> f64 {
        match SessionSpaceId::from_id(session_space_id as i64)
            .normalize_to_op_space(&self.compressor)
        {
            Err(err) => {
                self.set_hotpath_error(err.get_error_string());
                NAN
            }
            Ok(op_space_id) => op_space_id.id() as f64,
        }
    }

    pub fn normalize_to_session_space(&mut self, originator_token: f64, op_space_id: f64) -> f64 {
        let session_id = match self
            .compressor
            .get_session_id_from_session_token(originator_token as usize)
        {
            Err(e) => {
                self.set_hotpath_error(e.get_error_string());
                return NAN;
            }
            Ok(session_id) => session_id,
        };
        match OpSpaceId::from_id(op_space_id as i64)
            .normalize_to_session_space(session_id, &self.compressor)
        {
            Err(err) => {
                self.set_hotpath_error(err.get_error_string());
                NAN
            }
            Ok(session_space_id) => session_space_id.id() as f64,
        }
    }

    pub fn decompress(&mut self, id_to_decompress: f64) -> Result<String, JsError> {
        match SessionSpaceId::from_id(id_to_decompress as i64).decompress(&self.compressor) {
            Ok(stable_id) => Ok(stable_id.to_uuid_string()),
            Err(e) => Err(JsError::new(e.get_error_string())),
        }
    }

    pub fn recompress(&mut self, id_to_recompress: String) -> Result<f64, JsError> {
        let stable_id = match SessionId::from_uuid_string(&id_to_recompress) {
            Err(e) => return Err(JsError::new(e.get_error_string())),
            Ok(session_id) => StableId::from(session_id),
        };
        match stable_id.recompress(&self.compressor) {
            Ok(session_space_id) => Ok(session_space_id.id() as f64),
            Err(e) => Err(JsError::new(e.get_error_string())),
        }
    }

    pub fn serialize(&self, include_local_state: bool) -> Vec<u8> {
        self.compressor.serialize(include_local_state)
    }

    pub fn deserialize(
        &mut self,
        bytes: &[u8],
        session_id_string: String,
    ) -> Result<IdCompressor, JsError> {
        let session_id = match SessionId::from_uuid_string(&session_id_string) {
            Ok(id) => id,
            Err(e) => return Err(JsError::new(e.get_error_string())),
        };
        match IdCompressorCore::deserialize_with_session_id(bytes, || session_id) {
            Err(e) => Err(JsError::new(&e.get_error_string())),
            Ok(id_compressor) => Ok(IdCompressor {
                compressor: (id_compressor),
                error_string: (None),
            }),
        }
    }

    pub fn get_hotpath_error(&mut self) -> Option<String> {
        let error = self.error_string.clone();
        self.error_string = None;
        error
    }

    fn set_hotpath_error(&mut self, error: &str) {
        self.error_string = Some(String::from(error));
    }
}

#[wasm_bindgen]
pub struct InteropIdRange {
    token: f64,
    local: f64,
    count: f64,
}

#[wasm_bindgen]
impl InteropIdRange {
    #[wasm_bindgen(getter)]
    pub fn get_token(&self) -> f64 {
        self.token
    }
    #[wasm_bindgen(getter)]
    pub fn get_local(&self) -> f64 {
        self.local
    }
    #[wasm_bindgen(getter)]
    pub fn get_count(&self) -> f64 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use distributed_id_allocator::compressor::{NormalizationError, SessionTokenError};

    static DEFAULT_LOCAL: &str = "748540ca-b7c5-4c99-83ff-c1b8e02c09d6";

    #[test]
    #[should_panic]
    fn cluster_capacity_fract() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();
        _ = compressor.set_cluster_capacity(5.5 as f64);
    }

    #[test]
    #[should_panic]
    fn cluster_capacity_negative() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        _ = compressor.set_cluster_capacity(-2 as f64);
    }

    #[test]
    fn generate_next_id() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        assert_eq!(compressor.generate_next_id(), -1 as f64);
    }

    #[test]
    #[should_panic]
    fn get_token_invalid_uuid() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        _ = compressor.get_token(String::from("not_a_uuid")); // Errors at SessionId::from_uuid_string()
    }

    #[test]
    fn take_next_range() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        let id_1 = compressor.generate_next_id();
        let count_expected = 5;
        for _ in 1..count_expected {
            compressor.generate_next_id();
        }
        let interop_id_range = compressor.take_next_range();
        assert_eq!(interop_id_range.local, id_1);
        assert_eq!(interop_id_range.count, count_expected as f64);
    }

    #[test]
    fn take_next_range_empty() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        let interop_id_range = compressor.take_next_range();
        assert!(interop_id_range.local.is_nan());
        assert!(interop_id_range.count.is_nan());
    }

    #[test]
    fn finalize_range() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        for _ in 0..5 {
            compressor.generate_next_id();
        }
        let interop_id_range = compressor.take_next_range();

        assert!(compressor
            .finalize_range(
                interop_id_range.token,
                interop_id_range.local,
                interop_id_range.count
            )
            .is_ok());
    }

    #[test]
    fn normalize_to_op_space() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        for _ in 0..5 {
            compressor.generate_next_id();
        }
        let interop_id_range = compressor.take_next_range();
        _ = compressor.finalize_range(
            interop_id_range.token,
            interop_id_range.local,
            interop_id_range.count,
        );
        assert_eq!(compressor.normalize_to_op_space(-1 as f64), 0 as f64);
        assert_eq!(compressor.normalize_to_op_space(-5 as f64), 4 as f64);
        let new_final = compressor.generate_next_id();
        assert_eq!(compressor.normalize_to_op_space(new_final), new_final);

        assert!(compressor.normalize_to_op_space(-6 as f64).is_nan());
        assert_eq!(
            compressor.error_string,
            Some(String::from(
                NormalizationError::UnknownSessionSpaceId.get_error_string()
            ))
        );
    }

    #[test]
    fn normalize_to_session_space() {
        let mut compressor = IdCompressor::new(String::from(DEFAULT_LOCAL)).unwrap();

        for _ in 0..5 {
            compressor.generate_next_id();
        }
        let interop_id_range = compressor.take_next_range();
        _ = compressor.finalize_range(
            interop_id_range.token,
            interop_id_range.local,
            interop_id_range.count,
        );
        assert_eq!(
            compressor.normalize_to_session_space(0 as f64, 1.0),
            -2 as f64
        );
        assert!(compressor
            .normalize_to_session_space(3 as f64, -1.0)
            .is_nan());
        assert_eq!(
            compressor.error_string,
            Some(String::from(
                SessionTokenError::UnknownSessionToken.get_error_string()
            ))
        );
        assert!(compressor
            .normalize_to_session_space(0 as f64, 7.0)
            .is_nan());
    }
}
