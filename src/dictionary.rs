use std::fs::File;
use std::io::{BufRead, BufReader, Result};
use std::ops::ControlFlow;

/// Child index used for "no child": the root (index 0) is never a child
const NO_CHILD: u32 = 0;
/// `word_lengths` is a `u32` bitmask, so words must be shorter than this
const MAX_WORD_LENGTH: usize = 32;

#[derive(Default, Debug)]
pub struct TrieNode {
    pub children: [u32; 26],
    /// Bit `L` is set when a word of length `L` goes through this node
    pub word_lengths: u32,
    pub is_terminal: bool,
}

pub struct FlatTrie {
    nodes: Vec<TrieNode>,
}

impl Default for FlatTrie {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatTrie {
    pub fn new() -> Self {
        let mut trie = Self { nodes: Vec::with_capacity(1024) };
        trie.nodes.push(TrieNode::default()); // Root is always index 0
        trie
    }

    /// Inserts a lowercase, non-accented word (only `a..=z` bytes)
    pub fn insert(&mut self, word: &[u8]) {
        debug_assert!(word.iter().all(u8::is_ascii_lowercase), "word must only contain a-z");
        debug_assert!(word.len() < MAX_WORD_LENGTH, "word must be shorter than {MAX_WORD_LENGTH}");
        let length_bit = 1 << word.len();
        let mut current_idx = 0;
        self.nodes[current_idx].word_lengths |= length_bit;

        for &byte in word {
            let char_idx = (byte - b'a') as usize;

            let next_idx = match self.nodes[current_idx].children[char_idx] {
                NO_CHILD => {
                    let new_node_idx = self.nodes.len();
                    self.nodes.push(TrieNode::default());
                    self.nodes[current_idx].children[char_idx] = new_node_idx as u32;
                    new_node_idx
                }
                idx => idx as usize,
            };
            current_idx = next_idx;
            self.nodes[current_idx].word_lengths |= length_bit;
        }
        self.nodes[current_idx].is_terminal = true;
    }

    /// Builds the Trie from a file with one word per line. Words are lowercased,
    /// and lines containing anything other than ASCII letters (e.g. accents) are skipped,
    /// as well as words too long for `MAX_WORD_LENGTH`.
    pub fn from_file_path(path: &str) -> Result<Self> {
        let mut trie = Self::new();
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.len() >= MAX_WORD_LENGTH || !trimmed.bytes().all(|b| b.is_ascii_alphabetic()) {
                continue;
            }

            trie.insert(trimmed.to_ascii_lowercase().as_bytes());
        }

        Ok(trie)
    }

    /// Returns all words in the Trie matching a lowercase wildcard pattern (e.g., b"c.t")
    pub fn find_matches(&self, pattern: &[u8]) -> Vec<Vec<u8>> {
        let mut results = Vec::new();
        self.for_each_match(pattern, |word| {
            results.push(word.to_vec());
            ControlFlow::Continue(())
        });
        results
    }

    /// Counts the words matching `pattern`, stopping as soon as `limit` is reached
    pub fn count_matches(&self, pattern: &[u8], limit: usize) -> usize {
        let mut count = 0;
        if limit == 0 {
            return count;
        }
        self.for_each_match(pattern, |_| {
            count += 1;
            if count < limit { ControlFlow::Continue(()) } else { ControlFlow::Break(()) }
        });
        count
    }

    /// Calls `on_match` with each word matching `pattern`, until it returns `Break`
    fn for_each_match(&self, pattern: &[u8], mut on_match: impl FnMut(&[u8]) -> ControlFlow<()>) {
        if pattern.len() >= MAX_WORD_LENGTH {
            return;
        }
        let length_bit = 1 << pattern.len();
        if self.nodes[0].word_lengths & length_bit == 0 {
            return;
        }
        let mut prefix_buffer = Vec::with_capacity(pattern.len());
        let _ = self.visit_matches(0, pattern, length_bit, &mut prefix_buffer, &mut on_match);
    }

    /// Only walks into children holding words of the pattern's length (`length_bit`)
    fn visit_matches(
        &self,
        node_idx: usize,
        pattern: &[u8],
        length_bit: u32,
        prefix: &mut Vec<u8>,
        on_match: &mut impl FnMut(&[u8]) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        if pattern.is_empty() {
            if self.nodes[node_idx].is_terminal {
                return on_match(prefix);
            }
            return ControlFlow::Continue(());
        }

        let first = pattern[0];
        let rest = &pattern[1..];

        if first == b'.' {
            for (char_code, &child_idx) in self.nodes[node_idx].children.iter().enumerate() {
                if self.has_words_of_length(child_idx, length_bit) {
                    let letter = b'a' + char_code as u8;
                    prefix.push(letter);
                    let flow = self.visit_matches(child_idx as usize, rest, length_bit, prefix, on_match);
                    prefix.pop(); // Backtrack character buffer
                    flow?;
                }
            }
            ControlFlow::Continue(())
        } else {
            let child_idx = self.nodes[node_idx].children[(first - b'a') as usize];
            if self.has_words_of_length(child_idx, length_bit) {
                prefix.push(first);
                let flow = self.visit_matches(child_idx as usize, rest, length_bit, prefix, on_match);
                prefix.pop(); // Backtrack character buffer
                return flow;
            }
            ControlFlow::Continue(())
        }
    }

    fn has_words_of_length(&self, child_idx: u32, length_bit: u32) -> bool {
        child_idx != NO_CHILD && self.nodes[child_idx as usize].word_lengths & length_bit != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn finds_lowercase_matches() {
        let mut trie = FlatTrie::new();
        trie.insert(b"mer");
        trie.insert(b"mat");
        trie.insert(b"tir");

        assert_eq!(trie.find_matches(b"m.."), vec![b"mat".to_vec(), b"mer".to_vec()]);
        assert_eq!(trie.find_matches(b".ir"), vec![b"tir".to_vec()]);
        assert!(trie.find_matches(b"....").is_empty());
    }

    #[test]
    fn prunes_branches_by_word_length() {
        let mut trie = FlatTrie::new();
        trie.insert(b"mer");
        trie.insert(b"mers");
        trie.insert(b"merle");

        assert_eq!(trie.nodes[0].word_lengths, (1 << 3) | (1 << 4) | (1 << 5));
        assert_eq!(trie.find_matches(b"..."), vec![b"mer".to_vec()]);
        assert_eq!(trie.find_matches(b"...."), vec![b"mers".to_vec()]);
        assert_eq!(trie.find_matches(b"me..e"), vec![b"merle".to_vec()]);
        assert!(trie.find_matches(b"..").is_empty());
    }

    #[test]
    fn count_matches_stops_at_limit() {
        let mut trie = FlatTrie::new();
        for word in [b"mer", b"tir", b"mat", b"rer", b"sol", b"eau", b"ami"] {
            trie.insert(word);
        }

        assert_eq!(trie.count_matches(b"...", 100), 7);
        assert_eq!(trie.count_matches(b"...", 2), 2);
        assert_eq!(trie.count_matches(b"m..", 100), 2);
        assert_eq!(trie.count_matches(b"mer", 100), 1);
        assert_eq!(trie.count_matches(b"mex", 100), 0);
        assert_eq!(trie.count_matches(b"...", 0), 0);
    }

    #[test]
    fn from_file_path_lowercases_and_skips_accented_words() {
        let path = std::env::temp_dir().join("flechons_dictionary_test.txt");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "Mer\nété\nabaissa\n\ncafé").unwrap();

        let trie = FlatTrie::from_file_path(path.to_str().unwrap()).unwrap();

        assert_eq!(trie.find_matches(b"..."), vec![b"mer".to_vec()]);
        assert_eq!(trie.find_matches(b"abaissa"), vec![b"abaissa".to_vec()]);
        assert!(trie.find_matches(b"caf.").is_empty());
    }
}
