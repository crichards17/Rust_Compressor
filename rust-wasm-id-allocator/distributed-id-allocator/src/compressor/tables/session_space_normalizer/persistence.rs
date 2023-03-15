pub(crate) mod v1 {
    use crate::compressor::tables::session_space_normalizer::SessionSpaceNormalizer;
    use id_types::LocalId;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub(crate) struct PersistenceNormalizer {
        leading_locals: Vec<(LocalId, u64)>,
    }

    pub(crate) fn get_persistent_normalizer(
        session_space_normalizer: &SessionSpaceNormalizer,
    ) -> PersistenceNormalizer {
        PersistenceNormalizer {
            leading_locals: (session_space_normalizer.leading_locals.clone()),
        }
    }

    pub(crate) fn get_normalizer_from_persistent(
        persistent_normalizer: PersistenceNormalizer,
    ) -> SessionSpaceNormalizer {
        SessionSpaceNormalizer {
            leading_locals: (persistent_normalizer.leading_locals),
        }
    }
}
