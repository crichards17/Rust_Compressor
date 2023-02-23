use std::f64::NAN;

use distributed_id_allocator::{
    compressor::{ErrorEnum, IdCompressor as IdCompressorCore, IdRange},
    id_types::{LocalId, SessionId, SessionSpaceId, StableId},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct IdCompressor {
    compressor: IdCompressorCore,
    error_string: Option<String>,
}
/*
TODO:
- doc which methods may populate error field
*/

#[wasm_bindgen]
impl IdCompressor {
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
        debug_assert!(next_id <= 2 ^ 53 - 1 && next_id >= -2 ^ 53);
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

    /*
    pub fn finalize_range(
            &mut self,
            &IdRange {
                id: session_id,
                range,
            }: &IdRange,
        ) -> Result<(), FinalizationError>
     */

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
            id: id,
            range: Some((LocalId::new(range_base_local as i64), range_len as u64)),
        };
        match self.compressor.finalize_range(&id_range) {
            Err(e) => {
                self.set_error(e.get_error_string());
                return false;
            }
            Ok(_) => true,
        }
    }

    pub fn decompress(&mut self, id_to_decompress: f64) -> Option<String> {
        if !id_to_decompress.is_integer() {
            self.set_error("Non-integer ID passed to decompress.");
            return None;
        }
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

    /*

    set_cluster_capacity(

        take_next_range(&mut self) -> IdRange



        pub fn normalize_to_op_space(
            &self,
            compressor: &IdCompressor,
        ) -> Result<OpSpaceId, NormalizationError>

        pub fn normalize_to_session_space(
            &self,
            originator: SessionId,
            compressor: &IdCompressor,
        ) -> Result<SessionSpaceId, NormalizationError>


        */
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
    use super::*;

    #[test]
    fn test_i64_as_f64() {
        let num: i64 = -5;
        dbg!(num as f64);
    }
}
