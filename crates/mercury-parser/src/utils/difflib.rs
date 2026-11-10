//! A port of Python's `difflib.SequenceMatcher.ratio()`, as used via the
//! `difflib` package upstream.

use std::collections::HashMap;

/// Similarity of two strings in `[0, 1]`, comparing UTF-16 code units like
/// the JS port does. Uses difflib's defaults: no junk function, and the
/// "popular element" heuristic for sequences of 200 or more.
pub fn ratio(a: &str, b: &str) -> f64 {
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();
    let total = a.len() + b.len();
    if total == 0 {
        return 1.0;
    }

    let matches: usize = SequenceMatcher::new(&a, &b)
        .matching_blocks()
        .iter()
        .map(|&(_, _, size)| size)
        .sum();
    2.0 * matches as f64 / total as f64
}

struct SequenceMatcher<'a> {
    a: &'a [u16],
    b: &'a [u16],
    b2j: HashMap<u16, Vec<usize>>,
}

impl<'a> SequenceMatcher<'a> {
    fn new(a: &'a [u16], b: &'a [u16]) -> Self {
        let mut b2j: HashMap<u16, Vec<usize>> = HashMap::new();
        for (i, &elt) in b.iter().enumerate() {
            b2j.entry(elt).or_default().push(i);
        }

        // Purge popular elements, which are treated as junk.
        let n = b.len();
        if n >= 200 {
            let ntest = n / 100 + 1;
            b2j.retain(|_, indices| indices.len() <= ntest);
        }

        Self { a, b, b2j }
    }

    fn find_longest_match(
        &self,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
    ) -> (usize, usize, usize) {
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0);
        let mut j2len: HashMap<usize, usize> = HashMap::new();

        for i in alo..ahi {
            let mut new_j2len = HashMap::new();
            if let Some(indices) = self.b2j.get(&self.a[i]) {
                for &j in indices {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let k = j
                        .checked_sub(1)
                        .and_then(|prev| j2len.get(&prev))
                        .copied()
                        .unwrap_or(0)
                        + 1;
                    new_j2len.insert(j, k);
                    if k > bestsize {
                        besti = i + 1 - k;
                        bestj = j + 1 - k;
                        bestsize = k;
                    }
                }
            }
            j2len = new_j2len;
        }

        // Extend the match over equal elements that b2j skipped (popular
        // ones). Python's follow-up junk extension is a no-op without a junk
        // function.
        while besti > alo && bestj > blo && self.a[besti - 1] == self.b[bestj - 1] {
            besti -= 1;
            bestj -= 1;
            bestsize += 1;
        }
        while besti + bestsize < ahi
            && bestj + bestsize < bhi
            && self.a[besti + bestsize] == self.b[bestj + bestsize]
        {
            bestsize += 1;
        }

        (besti, bestj, bestsize)
    }

    fn matching_blocks(&self) -> Vec<(usize, usize, usize)> {
        let mut queue = vec![(0, self.a.len(), 0, self.b.len())];
        let mut blocks = Vec::new();

        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let (i, j, k) = self.find_longest_match(alo, ahi, blo, bhi);
            if k > 0 {
                blocks.push((i, j, k));
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i + k < ahi && j + k < bhi {
                    queue.push((i + k, ahi, j + k, bhi));
                }
            }
        }
        blocks.sort_unstable();

        // Collapse adjacent blocks.
        let mut collapsed: Vec<(usize, usize, usize)> = Vec::new();
        for (i, j, k) in blocks {
            if let Some(last) = collapsed.last_mut()
                && last.0 + last.2 == i
                && last.1 + last.2 == j
            {
                last.2 += k;
                continue;
            }
            collapsed.push((i, j, k));
        }
        collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_python_ratios() {
        // Values from Python's difflib.SequenceMatcher(None, a, b).ratio().
        assert!((ratio("abcd", "bcde") - 0.75).abs() < 1e-12);
        assert!(
            (ratio("http://example.com/foo", "http://example.com/foo/2") - 44.0 / 46.0).abs()
                < 1e-12
        );
        assert_eq!(ratio("", ""), 1.0);
        assert_eq!(ratio("abc", "xyz"), 0.0);
    }
}
