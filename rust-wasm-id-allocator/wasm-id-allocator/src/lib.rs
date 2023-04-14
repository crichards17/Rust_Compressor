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

// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         #[cfg(test)]
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use distributed_id_allocator::compressor::{
    ClusterCapacityError, IdCompressor as IdCompressorCore, IdRange,
};
use id_types::{OpSpaceId, SessionId, SessionSpaceId, StableId};
use std::f64::NAN;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug)]
pub struct IdCompressor {
    compressor: IdCompressorCore,
    error_string: Option<String>,
}

const MAX_DEFAULT_CLUSTER_CAPACITY: f64 = 2_i32.pow(11) as f64;
const NAN_UUID_U128: u128 = 0;

#[wasm_bindgen]
impl IdCompressor {
    pub fn get_default_cluster_capacity() -> f64 {
        IdCompressorCore::get_default_cluster_capacity() as f64
    }

    #[wasm_bindgen(constructor)]
    pub fn new(session_id_string: String) -> Result<IdCompressor, JsError> {
        Ok(IdCompressor {
            compressor: IdCompressorCore::new_with_session_id(SessionId::from_uuid_string(
                &session_id_string,
            )?),
            error_string: None,
        })
    }

    pub fn get_local_session_id(&self) -> String {
        self.compressor.get_local_session_id().to_uuid_string()
    }

    pub fn get_cluster_capacity(&self) -> f64 {
        self.compressor.get_cluster_capacity() as f64
    }

    pub fn set_cluster_capacity(&mut self, new_cluster_capacity: f64) -> Result<(), JsError> {
        if new_cluster_capacity.fract() != 0.0 || new_cluster_capacity < 0.0 {
            return Err(JsError::new(
                &ClusterCapacityError::InvalidClusterCapacity.to_string(),
            ));
        }
        if new_cluster_capacity > MAX_DEFAULT_CLUSTER_CAPACITY {
            return Err(JsError::new("Clusters must not exceed max cluster size."));
        }
        Ok(self
            .compressor
            .set_cluster_capacity(new_cluster_capacity as u64)?)
    }

    pub fn generate_next_id(&mut self) -> f64 {
        self.compressor.generate_next_id().id() as f64
    }

    pub fn get_token(&mut self, uuid_string: String) -> Result<f64, JsError> {
        Ok(self
            .compressor
            .get_session_token_from_session_id(SessionId::from_uuid_string(&uuid_string)?)
            .map(|x| x as f64)
            .unwrap_or(NAN))
    }

    pub fn take_next_range(&mut self) -> Option<InteropIds> {
        match self.compressor.take_next_range().range {
            Some((first_local_gen_count, count)) => Some(InteropIds {
                first_local_gen_count: first_local_gen_count as f64,
                count: count as f64,
            }),
            None => None,
        }
    }

    pub fn finalize_range(
        &mut self,
        session_id_str: String,
        range_base_count: f64,
        range_len: f64,
    ) -> Result<Option<InteropIdStats>, JsError> {
        self.compressor.finalize_range(&IdRange {
            id: SessionId::from_uuid_string(&session_id_str)?,
            range: Some((range_base_count as u64, range_len as u64)),
        })?;
        let stats = self.compressor.get_telemetry_stats();
        Ok(Some(InteropIdStats {
            eager_final_count: stats.eager_final_count as f64,
            local_id_count: stats.local_id_count as f64,
            expansion_count: stats.expansion_count as f64,
            cluster_creation_count: stats.cluster_creation_count as f64,
        }))
    }

    pub fn normalize_to_op_space(&mut self, session_space_id: f64) -> f64 {
        match &self
            .compressor
            .normalize_to_op_space(SessionSpaceId::from_id(session_space_id as i64))
        {
            Err(err) => {
                self.error_string = Some(err.to_string());
                NAN
            }
            Ok(op_space_id) => op_space_id.id() as f64,
        }
    }

    pub fn normalize_to_session_space(&mut self, op_space_id: f64, originator_token: f64) -> f64 {
        let session_id;
        // TS layer sends NAN token iff passing FinalId and a SessionId it has not tokenized.
        //  This can occur when normalizing an ID referenced by a client that has not finalized any IDs,
        //  and thus is not yet in the Sessions list.
        if originator_token.is_nan() {
            session_id = SessionId::from_uuid_u128(NAN_UUID_U128);
        } else {
            session_id = match self
                .compressor
                .get_session_id_from_session_token(originator_token as usize)
            {
                Err(err) => {
                    self.error_string = Some(err.to_string());
                    return NAN;
                }
                Ok(session_id) => session_id,
            };
        }
        match &self
            .compressor
            .normalize_to_session_space(OpSpaceId::from_id(op_space_id as i64), session_id)
        {
            Err(err) => {
                self.error_string = Some(err.to_string());
                NAN
            }
            Ok(session_space_id) => session_space_id.id() as f64,
        }
    }

    pub fn get_normalization_error_string(&mut self) -> Option<String> {
        let error = self.error_string.clone();
        self.error_string = None;
        error
    }

    pub fn decompress(&mut self, id_to_decompress: f64) -> Option<Vec<u8>> {
        let stable_id = self
            .compressor
            .decompress(SessionSpaceId::from_id(id_to_decompress as i64))
            .ok()?;
        let uuid_arr: [u8; 36] = stable_id.into();
        Some(Vec::from(uuid_arr))
    }

    pub fn recompress(&mut self, id_to_recompress: String) -> Option<f64> {
        Some(
            self.compressor
                .recompress(StableId::from(
                    SessionId::from_uuid_string(&id_to_recompress).ok()?,
                ))
                .ok()?
                .id() as f64,
        )
    }

    pub fn serialize(&self, include_local_state: bool) -> Vec<u8> {
        self.compressor.serialize(include_local_state)
    }

    pub fn deserialize(bytes: &[u8], session_id_string: String) -> Result<IdCompressor, JsError> {
        let session_id = SessionId::from_uuid_string(&session_id_string)?;
        Ok(IdCompressor {
            compressor: IdCompressorCore::deserialize_with_session_id_generator(bytes, || {
                session_id
            })?,
            error_string: None,
        })
    }
}

#[wasm_bindgen]
pub struct InteropIdStats {
    pub eager_final_count: f64,
    pub local_id_count: f64,
    pub expansion_count: f64,
    pub cluster_creation_count: f64,
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

#[wasm_bindgen]
pub struct TestOnly {}

#[wasm_bindgen]
impl TestOnly {
    #[wasm_bindgen]
    pub fn increment_uuid(_uuid_string: String, _offset: f64) -> Result<String, JsError> {
        #[cfg(debug_assertions)]
        return Ok(
            (StableId::from(SessionId::from_uuid_string(&_uuid_string).unwrap())
                + (_offset as u64))
                .into(),
        );
        #[cfg(not(debug_assertions))]
        Err(JsError::new("Not supported in release."))
    }

    #[wasm_bindgen]
    pub fn compressor_equals(
        _a: &IdCompressor,
        _b: &IdCompressor,
        _compare_local_state: bool,
    ) -> Result<bool, JsError> {
        #[cfg(debug_assertions)]
        return Ok(_a
            .compressor
            .equals_test_only(&_b.compressor, _compare_local_state));
        #[cfg(not(debug_assertions))]
        Err(JsError::new("Not supported in release."))
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
        let mut compressor = IdCompressor::new(String::from(_STABLE_ID_1)).ok().unwrap();
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
        } = interop_id_range.unwrap();
        _ = compressor.finalize_range(
            compressor.get_local_session_id().into(),
            first_local_gen_count,
            count,
        )
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
        _ = compressor.get_token(String::from("not_a_uuid"));
    }

    #[test]
    fn take_next_range() {
        let (mut compressor, generated_ids) = initialize_compressor();
        let interop_id_range = compressor.take_next_range();
        let InteropIds {
            first_local_gen_count,
            count,
        } = interop_id_range.unwrap();
        assert_eq!(
            LocalId::from_generation_count(first_local_gen_count as u64).id() as f64,
            generated_ids[0]
        );
        assert_eq!(count, generated_ids.len() as f64);
    }

    #[test]
    fn take_next_range_empty() {
        let mut compressor = IdCompressor::new(String::from(_STABLE_ID_1)).ok().unwrap();
        let interop_id_range = compressor.take_next_range();
        assert!(interop_id_range.is_none());
    }

    #[test]
    fn finalize_range() {
        let (mut compressor, _) = initialize_compressor();
        let interop_id_range = compressor.take_next_range();
        let InteropIds {
            first_local_gen_count,
            count,
        } = interop_id_range.unwrap();
        assert!(compressor
            .finalize_range(
                compressor.get_local_session_id().into(),
                first_local_gen_count,
                count
            )
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
                    compressor
                        .compressor
                        .get_session_token_from_session_id(
                            compressor.compressor.get_local_session_id()
                        )
                        .ok()
                        .unwrap() as f64
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
            Some(NormalizationError::UnknownSessionSpaceId.to_string())
        );
    }

    #[test]
    fn normalize_to_session_space() {
        let (mut compressor, _) = initialize_compressor();
        finalize_compressor(&mut compressor);
        assert_eq!(
            compressor.normalize_to_session_space(
                1.0,
                compressor
                    .compressor
                    .get_session_token_from_session_id(compressor.compressor.get_local_session_id())
                    .ok()
                    .unwrap() as f64
            ),
            -2 as f64
        );
        assert!(compressor
            .normalize_to_session_space(-3 as f64, 4.0)
            .is_nan());
        assert_eq!(
            compressor.error_string,
            Some(SessionTokenError::UnknownSessionToken.to_string())
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
            let buff = compressor.decompress(id).unwrap();
            let uuid_str = String::from_utf8(buff).unwrap();
            assert_eq!(uuid_str, String::from(base_stable + expected_offset));
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
        let session_id =
            (StableId::from(SessionId::from_uuid_string(_STABLE_ID_1).unwrap()) + 1).into();
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
        assert!(compressor
            .compressor
            .equals_test_only(&compressor_serialized_deserialized.compressor, false))
    }
}
