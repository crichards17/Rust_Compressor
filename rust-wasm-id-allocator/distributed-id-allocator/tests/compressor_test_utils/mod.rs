use distributed_id_allocator::compressor::IdCompressor;
use id_types::*;

const DEFAULT_CLUSTER_CAPACITY: u64 = 5;

pub struct CompressorFactory {
    compressors: Vec<IdCompressor>,
}

impl<'a> CompressorFactory {
    pub fn new() -> Self {
        CompressorFactory {
            compressors: Vec::new(),
        }
    }

    pub fn get_compressor_count(&self) -> usize {
        self.compressors.len()
    }

    pub fn create_compressor(
        &mut self,
        client: Client,
        cluster_capacity: Option<u64>,
    ) -> &mut IdCompressor {
        let session_id = SessionId::from_uuid_string(client.get_session_id()).unwrap();
        self.create_compressor_with_session(session_id, cluster_capacity)
    }

    pub fn create_compressor_with_session(
        &mut self,
        session_id: SessionId,
        cluster_capacity: Option<u64>,
    ) -> &mut IdCompressor {
        let mut compressor = IdCompressor::new_with_session_id(session_id);
        _ = compressor.set_cluster_capacity(DEFAULT_CLUSTER_CAPACITY);
        if let Some(capacity) = cluster_capacity {
            if compressor.set_cluster_capacity(capacity).is_err() {
                panic!(
                    "Invalid cluster capacity passed to create_compressor_with_session: {}",
                    capacity
                );
            };
        }
        let index = self.compressors.len();
        self.compressors.push(compressor);
        &mut self.compressors[index]
    }
}

const _CLIENT_SESSION_IDS: &'static [&'static str] = &[
    "0002c79e-b536-4776-b000-000266c252d5",
    "082533b9-6d05-4068-a008-fe2cc43543f2",
    "0002c79e-b536-4776-b000-000266c252d3",
];

pub enum Client {
    Client1 = 0,
    Client2 = 1,
    Client3 = 2,
}

impl Client {
    pub fn get_session_id(self) -> &'static str {
        _CLIENT_SESSION_IDS[self as usize]
    }
}
