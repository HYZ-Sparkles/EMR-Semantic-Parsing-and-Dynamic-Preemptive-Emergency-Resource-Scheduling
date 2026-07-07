// 手工实现的字典树 (Trie)。只支持中文字符 (char) 作为边。
// 用 HashMap 存子节点，便于处理 0x4E00-0x9FA5 这种稀疏字符空间。
// 节点 is_end=true 表示该节点是一个完整词条，weight 表示该词的累计危险分。

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
    weight: u32,
    term: Option<String>, // 仅在 is_end=true 时填充，用于回显匹配到的词
}

#[derive(Default, Debug)]
pub struct Trie {
    root: TrieNode,
    size: usize,    // 记录词条总数
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    pub word: String,
    pub weight: u32,
}

impl Trie {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, word: &str, weight: u32) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_insert_with(TrieNode::default);
        }
        if !node.is_end {
            self.size += 1;
        }
        node.is_end = true;
        node.weight = weight;
        node.term = Some(word.to_string());
    }

    /// 在文本里扫描所有词条，返回 match(起始字节位置, 结束字节位置(不含), 词, 权重) 列表。
    /// 不允许同一位置匹配多个长度不同的词（如"心"和"心肌梗死"）。
    pub fn scan(&self, text: &str) -> Vec<Match> {
        let mut hits = Vec::new();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        for i in 0..chars.len() {
            let mut node = &self.root;
            for j in i..chars.len() {  
                let (byte_idx, ch) = chars[j];
                match node.children.get(&ch) {
                    Some(child) => {
                        node = child;
                        if node.is_end {
                            let end = chars.get(j + 1).map(|(b, _)| *b).unwrap_or(text.len());
                            hits.push(Match {
                                start: byte_idx,
                                end,
                                word: node.term.clone().unwrap_or_default(),
                                weight: node.weight,
                            });
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
        // 去重: 同一个 (start,end) 保留权重最高的 (防御性)
        hits.sort_by_key(|m| (m.start, m.end, std::cmp::Reverse(m.weight)));
        hits.dedup_by(|a, b| a.start == b.start && a.end == b.end);
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insert_and_scan() {
        let mut t = Trie::new();
        t.insert("胸痛", 11);
        t.insert("胸痛伴", 14);
        t.insert("昏迷", 14);
        let m = t.scan("突发胸痛伴昏迷1小时");
        let words: Vec<_> = m.iter().map(|x| x.word.as_str()).collect();
        assert!(words.contains(&"胸痛"));
        assert!(words.contains(&"昏迷"));
    }
}
