#![deny(
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    // missing_debug_implementations,
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    // unused_results
)]

use distributed_id_allocator::compressor::{ErrorEnum, IdCompressor as IdCompressorCore, IdRange};
use id_types::{OpSpaceId, SessionId, SessionSpaceId, StableId};
use std::f64::NAN;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, PartialEq)]
pub struct IdCompressor {
    compressor: IdCompressorCore,
    error_string: Option<String>,
}

const BINARY_BASE: i64 = 2;
const MAX_SAFE_INTEGER: i64 = BINARY_BASE.pow(53) - 1;

#[wasm_bindgen]
impl IdCompressor {
    #[wasm_bindgen]
    pub fn get_default_cluster_capacity() -> f64 {
        IdCompressorCore::get_default_cluster_capacity() as f64
    }

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
            Some((first_local_gen_count, count)) => InteropIdRange {
                token,
                ids: Some(InteropIds {
                    first_local_gen_count: first_local_gen_count as f64,
                    count: count as f64,
                }),
            },
            None => InteropIdRange { token, ids: None },
        }
    }

    pub fn finalize_range(
        &mut self,
        session_token: f64,
        range_base_count: f64,
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
                range: Some((range_base_count as u64, range_len as u64)),
            })
            .map_err(|e| JsError::new(e.get_error_string()))
    }

    pub fn normalize_to_op_space(&mut self, session_space_id: f64) -> f64 {
        match &self
            .compressor
            .normalize_to_op_space(SessionSpaceId::from_id(session_space_id as i64))
        {
            Err(err) => {
                self.set_error_string(err.get_error_string());
                NAN
            }
            Ok(op_space_id) => op_space_id.id() as f64,
        }
    }

    pub fn normalize_to_session_space(&mut self, op_space_id: f64, originator_token: f64) -> f64 {
        let session_id = match self
            .compressor
            .get_session_id_from_session_token(originator_token as usize)
        {
            Err(e) => {
                self.set_error_string(e.get_error_string());
                return NAN;
            }
            Ok(session_id) => session_id,
        };
        match &self
            .compressor
            .normalize_to_session_space(OpSpaceId::from_id(op_space_id as i64), session_id)
        {
            Err(err) => {
                self.set_error_string(err.get_error_string());
                NAN
            }
            Ok(session_space_id) => session_space_id.id() as f64,
        }
    }

    pub fn decompress(&mut self, id_to_decompress: f64) -> Option<String> {
        match &self
            .compressor
            .decompress(SessionSpaceId::from_id(id_to_decompress as i64))
        {
            Ok(stable_id) => Some(stable_id.to_uuid_string()),
            Err(e) => {
                self.set_error_string(e.get_error_string());
                None
            }
        }
    }

    pub fn recompress(&mut self, id_to_recompress: String) -> Option<f64> {
        let stable_id = match SessionId::from_uuid_string(&id_to_recompress) {
            Err(e) => {
                self.set_error_string(e.get_error_string());
                return None;
            }
            Ok(session_id) => StableId::from(session_id),
        };
        match &self.compressor.recompress(stable_id) {
            Ok(session_space_id) => Some(session_space_id.id() as f64),
            Err(e) => {
                self.set_error_string(e.get_error_string());
                None
            }
        }
    }

    pub fn serialize(&self, include_local_state: bool) -> Vec<u8> {
        self.compressor.serialize(include_local_state)
    }

    pub fn deserialize(bytes: &[u8], session_id_string: String) -> Result<IdCompressor, JsError> {
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

    pub fn get_error_string(&mut self) -> Option<String> {
        let error = self.error_string.clone();
        self.error_string = None;
        error
    }

    fn set_error_string(&mut self, error: &str) {
        self.error_string = Some(String::from(error));
    }
}

#[wasm_bindgen]
pub struct InteropIdRange {
    token: f64,
    ids: Option<InteropIds>,
}

#[wasm_bindgen]
impl InteropIdRange {
    #[wasm_bindgen(getter)]
    pub fn token(&self) -> f64 {
        self.token
    }

    #[wasm_bindgen(getter)]
    pub fn ids(&self) -> Option<InteropIds> {
        self.ids
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct InteropIds {
    first_local_gen_count: f64,
    count: f64,
}

#[wasm_bindgen]
impl InteropIds {
    #[wasm_bindgen(getter)]
    pub fn first_local_gen_count(&self) -> f64 {
        self.first_local_gen_count
    }
    #[wasm_bindgen(getter)]
    pub fn count(&self) -> f64 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use distributed_id_allocator::compressor::{NormalizationError, SessionTokenError};
    use id_types::LocalId;

    const _STABLE_ID_1: &str = "748540ca-b7c5-4c99-83ff-c1b8e02c09d6";
    const _STABLE_ID_2: &str = "0002c79e-b536-4776-b000-000266c252d5";

    fn initialize_compressor() -> (IdCompressor, Vec<f64>) {
        let mut compressor = IdCompressor::new(String::from(_STABLE_ID_1)).unwrap();
        let mut generated_ids: Vec<f64> = Vec::new();
        for _ in 0..5 {
            generated_ids.push(compressor.generate_next_id());
        }
        (compressor, generated_ids)
    }

    fn finalize_compressor(compressor: &mut IdCompressor) {
        let interop_id_range = compressor.take_next_range();
        let InteropIds {
            first_local_gen_count,
            count,
        } = interop_id_range.ids.unwrap();
        _ = compressor.finalize_range(interop_id_range.token, first_local_gen_count, count)
    }

    #[test]
    #[should_panic]
    fn cluster_capacity_fract() {
        let (mut compressor, _) = initialize_compressor();
        _ = compressor.set_cluster_capacity(5.5);
    }

    #[test]
    #[should_panic]
    fn cluster_capacity_negative() {
        let (mut compressor, _) = initialize_compressor();
        _ = compressor.set_cluster_capacity(-2 as f64);
    }

    #[test]
    fn generate_next_id() {
        let (mut compressor, generated_ids) = initialize_compressor();
        assert_eq!(
            compressor.generate_next_id(),
            generated_ids[generated_ids.len() - 1] - 1.0
        );
    }

    #[test]
    #[should_panic]
    fn get_token_invalid_uuid() {
        let (mut compressor, _) = initialize_compressor();
        _ = compressor.get_token(String::from("not_a_uuid")); // Errors at SessionId::from_uuid_string()
    }

    #[test]
    fn take_next_range() {
        let (mut compressor, generated_ids) = initialize_compressor();
        let interop_id_range = compressor.take_next_range();
        let InteropIds {
            first_local_gen_count,
            count,
        } = interop_id_range.ids.unwrap();
        assert_eq!(interop_id_range.token, 0.0);
        assert_eq!(
            LocalId::from_generation_count(first_local_gen_count as u64).id() as f64,
            generated_ids[0]
        );
        assert_eq!(count, generated_ids.len() as f64);
    }

    #[test]
    fn take_next_range_empty() {
        let mut compressor = IdCompressor::new(String::from(_STABLE_ID_1)).unwrap();
        let interop_id_range = compressor.take_next_range();
        assert!(interop_id_range.ids.is_none());
    }

    #[test]
    fn finalize_range() {
        let (mut compressor, _) = initialize_compressor();
        let interop_id_range = compressor.take_next_range();
        let InteropIds {
            first_local_gen_count,
            count,
        } = interop_id_range.ids.unwrap();
        assert!(compressor
            .finalize_range(interop_id_range.token, first_local_gen_count, count)
            .is_ok());
    }

    #[test]
    fn normalize_to_op_space() {
        let (mut compressor, generated_ids) = initialize_compressor();
        finalize_compressor(&mut compressor);
        let id_count = generated_ids.len();
        for id in generated_ids {
            let op_space_id = compressor.normalize_to_op_space(id);
            assert_eq!(
                compressor.normalize_to_session_space(
                    op_space_id,
                    compressor.compressor.get_local_session_token() as f64
                ),
                id
            );
        }
        let new_final = compressor.generate_next_id();
        assert_eq!(compressor.normalize_to_op_space(new_final), new_final);

        assert!(compressor
            .normalize_to_op_space(0.0 - (id_count as f64) - 1.0)
            .is_nan());

        assert_eq!(
            compressor.error_string,
            Some(String::from(
                NormalizationError::UnknownSessionSpaceId.get_error_string()
            ))
        );
    }

    #[test]
    fn normalize_to_session_space() {
        let (mut compressor, _) = initialize_compressor();
        finalize_compressor(&mut compressor);
        assert_eq!(
            compressor.normalize_to_session_space(
                1.0,
                compressor.compressor.get_local_session_token() as f64
            ),
            -2 as f64
        );
        assert!(compressor
            .normalize_to_session_space(-3 as f64, 4.0)
            .is_nan());
        assert_eq!(
            compressor.error_string,
            Some(String::from(
                SessionTokenError::UnknownSessionToken.get_error_string()
            ))
        );
        assert!(compressor.normalize_to_session_space(7.0, 0.0).is_nan());
    }

    #[test]
    fn decompress_invalid() {
        let (mut compressor, _) = initialize_compressor();

        assert!(compressor.decompress(1.0).is_none());
    }

    #[test]
    fn decompress() {
        let (mut compressor, generated_ids) = initialize_compressor();
        finalize_compressor(&mut compressor);
        let session_id = compressor.compressor.get_local_session_id();
        let base_stable = StableId::from(session_id);
        for id in generated_ids {
            let expected_offset = ((id * -1.0) - 1.0) as u64;
            assert_eq!(
                compressor.decompress(id).unwrap(),
                (base_stable + expected_offset).to_uuid_string()
            );
        }
    }
    #[test]
    fn recompress_invalid_uuid_string() {
        let (mut compressor, _) = initialize_compressor();

        assert!(compressor
            .recompress(String::from("invalid_uuid"))
            .is_none());
    }

    #[test]
    fn recompress_unknown_uuid() {
        let (mut compressor, _) = initialize_compressor();

        assert!(compressor.recompress(String::from(_STABLE_ID_2)).is_none());
    }

    #[test]
    fn recompress() {
        let (mut compressor, _) = initialize_compressor();
        finalize_compressor(&mut compressor);
        let session_id = (StableId::from(SessionId::from_uuid_string(_STABLE_ID_1).unwrap()) + 1)
            .to_uuid_string();
        assert!(compressor.recompress(session_id).is_some());
    }

    #[test]
    #[should_panic]
    fn deserialize_invalid() {
        let bytes: &[u8] = &[1, 2, 1, 0, 1];
        _ = IdCompressor::deserialize(bytes, String::from(_STABLE_ID_1));
    }

    #[test]
    fn serialize_deserialize() {
        let (mut compressor, _) = initialize_compressor();
        finalize_compressor(&mut compressor);
        compressor.generate_next_id();
        let serialized_local = compressor.serialize(true);
        assert!(IdCompressor::deserialize(&serialized_local, String::from(_STABLE_ID_1)).is_ok());
        let serialized_final = compressor.serialize(false);
        assert!(IdCompressor::deserialize(&serialized_final, String::from(_STABLE_ID_2)).is_ok());
        let compressor_serialized_deserialized =
            IdCompressor::deserialize(&serialized_local, String::from(_STABLE_ID_1))
                .ok()
                .unwrap();
        assert_eq!(compressor, compressor_serialized_deserialized)
    }
}
