use crate::id_types::LocalId;
use std::cmp::Ordering;

#[derive(Debug)]
pub struct SessionSpaceNormalizer {
    leading_locals: Vec<(LocalId, u64)>,
}

impl SessionSpaceNormalizer {
    pub fn new() -> Self {
        Self {
            leading_locals: Vec::new(),
        }
    }

    pub fn add_local_range(&mut self, base_local: LocalId, count: u64) {
        if let Some((last_local, last_count)) = self.leading_locals.last_mut() {
            if *last_local - *last_count == base_local {
                *last_count += count;
                return;
            }
        }
        self.leading_locals.push((base_local, count));
    }

    pub fn contains(&self, query: LocalId) -> bool {
        self.leading_locals
            .binary_search_by(|(current_local, current_count)| {
                if &query > current_local {
                    return Ordering::Less;
                } else if query < *current_local - *current_count {
                    return Ordering::Greater;
                } else {
                    Ordering::Equal
                }
            })
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let session_space_normalizer = SessionSpaceNormalizer::new();
        assert!(!session_space_normalizer.contains(LocalId::new(-1)));
    }

    #[test]
    fn test_single() {
        let mut session_space_normalizer = SessionSpaceNormalizer::new();
        session_space_normalizer.add_local_range(LocalId::new(-1), 5);
        assert!(session_space_normalizer.contains(LocalId::new(-1)));
        assert!(session_space_normalizer.contains(LocalId::new(-2)));
    }

    #[test]
    fn test_discontiguous() {
        let mut session_space_normalizer = SessionSpaceNormalizer::new();
        session_space_normalizer.add_local_range(LocalId::new(-1), 2);
        session_space_normalizer.add_local_range(LocalId::new(-6), 4);
        session_space_normalizer.add_local_range(LocalId::new(-15), 1);
        assert!(!session_space_normalizer.contains(LocalId::new(-11)));
        assert!(!session_space_normalizer.contains(LocalId::new(-4)));
        assert!(session_space_normalizer.contains(LocalId::new(-7)));
    }

    #[test]
    fn test_contiguous() {
        let mut session_space_normalizer = SessionSpaceNormalizer::new();
        session_space_normalizer.add_local_range(LocalId::new(-1), 2);
        session_space_normalizer.add_local_range(LocalId::new(-3), 4);
        session_space_normalizer.add_local_range(LocalId::new(-15), 1);
        assert!(session_space_normalizer.contains(LocalId::new(-1)));
        assert!(session_space_normalizer.contains(LocalId::new(-4)));
        assert!(session_space_normalizer.contains(LocalId::new(-6)));
        assert!(session_space_normalizer.contains(LocalId::new(-7)));
        assert!(session_space_normalizer.contains(LocalId::new(-15)));
    }
}
