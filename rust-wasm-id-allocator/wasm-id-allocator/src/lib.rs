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

#[wasm_bindgen]
impl IdCompressor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> IdCompressor {
        IdCompressor {
            compressor: IdCompressorCore::new(),
            error_string: None,
        }
    }

    pub fn set_cluster_capacity(&mut self, new_cluster_capacity: f64) -> bool {
        if !new_cluster_capacity.is_integer() {
            self.set_error("Non-integer cluster size.");
            return false;
        }
        if let Err(err) = self
            .compressor
            .set_cluster_capacity(new_cluster_capacity as u64)
        {
            self.set_error(err.get_error_string());
            false
        } else {
            true
        }
    }

    pub fn generate_next_id(&mut self) -> f64 {
        let next_id = self.compressor.generate_next_id().id();
        debug_assert!(next_id <= (2 as i64).pow(53) - 1 && next_id >= -(2 as i64).pow(53));
        next_id as f64
    }

    pub fn get_token(&mut self, uuid_string: String) -> Option<f64> {
        let session_id = match SessionId::from_uuid_string(&uuid_string) {
            Err(e) => {
                self.set_error(e.get_error_string());
                return None;
            }
            Ok(session_id) => session_id,
        };
        match self
            .compressor
            .get_session_token_from_session_id(session_id)
        {
            Err(e) => {
                self.set_error(e.get_error_string());
                return None;
            }
            Ok(token) => Some(token as f64),
        }
    }

    pub fn take_next_range(&mut self) -> Option<InteropIdRange> {
        let id_range = self.compressor.take_next_range();
        let id_token = match self
            .compressor
            .get_session_token_from_session_id(id_range.id)
        {
            Err(e) => {
                self.set_error(e.get_error_string());
                return None;
            }
            Ok(token) => token,
        };
        match id_range.range {
            Some((local, count)) => Some(InteropIdRange {
                token: id_token as f64,
                local: local.id() as f64,
                count: count as f64,
            }),
            None => Some(InteropIdRange {
                token: id_token as f64,
                local: NAN,
                count: NAN,
            }),
        }
    }

    pub fn finalize_range(
        &mut self,
        session_token: f64,
        range_base_local: f64,
        range_len: f64,
    ) -> bool {
        if !session_token.is_integer()
            || !range_base_local.is_integer()
            || !range_len.is_integer()
            || session_token < 0.0
            || range_base_local > -1.0
            || range_len <= 0.0
        {
            self.set_error("Invalid Range parameter(s).");
            return false;
        }
        let id = match self
            .compressor
            .get_session_id_from_session_token(session_token as usize)
        {
            Err(e) => {
                self.set_error(e.get_error_string());
                return false;
            }
            Ok(session_id) => session_id,
        };
        let id_range = IdRange {
            id,
            range: Some((LocalId::new(range_base_local as i64), range_len as u64)),
        };
        if let Err(e) = self.compressor.finalize_range(&id_range) {
            self.set_error(e.get_error_string());
            false
        } else {
            true
        }
    }

    pub fn normalize_to_op_space(&mut self, session_space_id: f64) -> f64 {
        if !session_space_id.is_integer() {
            self.set_error("Non-integer session space ID.");
            return NAN;
        };
        match SessionSpaceId::from_id(session_space_id as i64)
            .normalize_to_op_space(&self.compressor)
        {
            Err(err) => {
                self.set_error(err.get_error_string());
                NAN
            }
            Ok(op_space_id) => op_space_id.id() as f64,
        }
    }

    pub fn normalize_to_session_space(&mut self, originator_token: f64, op_space_id: f64) -> f64 {
        if !op_space_id.is_integer() || !originator_token.is_integer() {
            self.set_error("Non-integer inputs.");
            NAN
        } else {
            let session_id = match self
                .compressor
                .get_session_id_from_session_token(originator_token as usize)
            {
                Err(e) => {
                    self.set_error(e.get_error_string());
                    return NAN;
                }
                Ok(session_id) => session_id,
            };
            match OpSpaceId::from_id(op_space_id as i64)
                .normalize_to_session_space(session_id, &self.compressor)
            {
                Err(err) => {
                    self.set_error(err.get_error_string());
                    NAN
                }
                Ok(session_space_id) => session_space_id.id() as f64,
            }
        }
    }

    pub fn decompress(&mut self, id_to_decompress: f64) -> Option<String> {
        if !id_to_decompress.is_integer() {
            self.set_error("Non-integer ID passed to decompress.");
            return None;
        };
        let session_space_id = SessionSpaceId::from_id(id_to_decompress as i64);
        match session_space_id.decompress(&self.compressor) {
            Ok(stable_id) => Some(stable_id.to_uuid_string()),
            Err(e) => {
                self.set_error(e.get_error_string());
                None
            }
        }
    }

    pub fn recompress(&mut self, id_to_recompress: String) -> f64 {
        let stable_id = match SessionId::from_uuid_string(&id_to_recompress) {
            Err(e) => {
                self.set_error(e.get_error_string());
                return NAN;
            }
            Ok(session_id) => StableId::from(session_id),
        };
        match stable_id.recompress(&self.compressor) {
            Ok(session_space_id) => session_space_id.id() as f64,
            Err(e) => {
                self.set_error(e.get_error_string());
                NAN
            }
        }
    }

    // Note: bindgen does not allow returning a reference slice
    pub fn serialize(&self, include_local_state: bool) -> Vec<u8> {
        self.compressor.serialize(include_local_state)
    }

    pub fn deserialize(&mut self, bytes: &[u8]) -> Option<IdCompressor> {
        match IdCompressorCore::deserialize(bytes) {
            Err(e) => {
                self.set_error(&e.get_error_string());
                None
            }
            Ok(id_compressor) => Some(IdCompressor {
                compressor: (id_compressor),
                error_string: (None),
            }),
        }
    }

    pub fn get_error(&mut self) -> String {
        let error = match &self.error_string {
            None => String::from(""),
            Some(e) => (*e).clone(),
        };
        self.error_string = None;
        error
    }

    fn set_error(&mut self, error: &str) {
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

trait IsInt {
    fn is_integer(&self) -> bool;
}

impl IsInt for f64 {
    fn is_integer(&self) -> bool {
        self.fract() == 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::IsInt;

    #[test]
    fn is_int() {
        assert!(0.0.is_integer());
        assert!(1.0.is_integer());
        assert!((-1.0).is_integer());
        assert!(((2 as u64).pow(52) as f64).is_integer());
        assert!((((2 as u64).pow(53) - 1) as f64).is_integer());
        assert!(!0.1.is_integer());
        assert!(!(-0.1).is_integer());
        dbg!((((2 as u64).pow(54) + 3) as f64).is_integer());
        assert!((((2 as u64).pow(54) + 3) as f64).is_integer());
    }
}
