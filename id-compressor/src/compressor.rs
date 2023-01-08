/*
on compressor:

generate_next_id

take_next_block

finalize_block

serialize

deserialize

----------------------
on id types:

decompress

recompress

normalize_to_op_space

normalize_to_session_space

*/
use super::id_types::*;

pub struct IdCompressor {
    // state
    session_id: SessionId,
    local_id_count: i64,
    last_taken_local_id_count: i64,
}

impl IdCompressor {
    // TODO: Update to match final state
    pub fn new() -> IdCompressor {
        IdCompressor {
            session_id: SessionId::new(),
            local_id_count: 0,
            last_taken_local_id_count: 0,
        }
    }

    // TODO: Eager finals
    pub fn generate_next_id(&mut self) -> SessionSpaceId {
        self.local_id_count += 1;
        SessionSpaceId {
            id: -self.local_id_count,
        }
    }

    pub fn take_next_block(&self) -> IdBlock {
        IdBlock {
            id: self.session_id,
            block: if self.local_id_count == self.last_taken_local_id_count {
                None
            } else {
                let count = self.local_id_count - self.last_taken_local_id_count;
                assert!(
                    count > 0,
                    "Must only allocate a positive number of IDs. Count was {}",
                    count
                );
                Some((LocalId::new(-self.last_taken_local_id_count), count))
            },
        }
    }
}

pub struct IdBlock {
    pub(crate) id: SessionId,
    pub(crate) block: Option<(LocalId, i64)>,
}
