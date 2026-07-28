// 手工实现的大顶堆 + 位置映射 (index map)。
// 1. 堆按 score 降序排列，分数越高越靠上；分数相同时按 arrived_tick 升序 (先到先出)。
// 2. 位置映射: id -> 堆内下标，使得 increase_key / find / remove 都能在 O(log N) 完成。

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,        // 患者 id (UUID)
    pub score: u32,         // 当前权重
    pub arrived_tick: u64,  // 入队时刻，用于 FIFO 兜底
    pub heap_idx: usize,    // 在堆中的下标，Invalid 时为 usize::MAX
}

pub const INVALID: usize = usize::MAX;

#[derive(Default, Debug)]
pub struct IndexedMaxHeap {
    data: Vec<Entry>,              // data[0] = 堆顶
    pos: HashMap<String, usize>,   // id -> data 下标
}

impl IndexedMaxHeap {
    pub fn len(&self) -> usize { self.data.len() }
    pub fn is_empty(&self) -> bool { self.data.is_empty() }
    pub fn peek(&self) -> Option<&Entry> { self.data.first() }

    pub fn contains(&self, id: &str) -> bool { self.pos.contains_key(id) }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.pos.get(id).and_then(|i| self.data.get(*i))
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Entry> {
        let i = *self.pos.get(id)?;
        self.data.get_mut(i)
    }

    /// O(log N) 插入。
    pub fn push(&mut self, mut e: Entry) {
        let id = e.id.clone();
        e.heap_idx = self.len();
        self.data.push(e);
        self.pos.insert(id, self.data.len() - 1);
        self.sift_up(self.data.len() - 1);
    }

    /// O(log N) 弹出堆顶。
    pub fn pop(&mut self) -> Option<Entry> {
        if self.data.is_empty() { return None; }
        let last = self.data.len() - 1;
        self.swap(0, last);
        let mut top = self.data.pop().unwrap();
        self.pos.remove(&top.id);
        top.heap_idx = INVALID;
        if !self.data.is_empty() {
            // 修复刚换到根的旧 last
            self.data[0].heap_idx = 0;
            self.pos.insert(self.data[0].id.clone(), 0);
            self.sift_down(0);
        }
        Some(top)
    }

    /// O(log N) 删除指定 id。
    pub fn remove(&mut self, id: &str) -> Option<Entry> {
        let i = match self.pos.get(id).copied() {
            Some(i) => i,
            None => return None,
        };
        let last = self.data.len() - 1;
        self.swap(i, last);
        let removed = self.data.pop().unwrap();
        self.pos.remove(&removed.id);
        if i < self.data.len() {
            // 移过来的元素既可能需要上浮也可能需要下沉
            self.sift_up(i);
            self.sift_down(i);
        }
        Some(removed)
    }

    /// O(log N) 调整分数 (任意方向)。
    pub fn update_score(&mut self, id: &str, new_score: u32) -> bool {
        if let Some(e) = self.get_mut(id) {
            let old = e.score;
            e.score = new_score;
            let i = e.heap_idx;
            if new_score > old { self.sift_up(i); } else { self.sift_down(i); }
            return true;
        }
        false
    }

    /// 返回按堆顺序的快照 (从顶到底)。O(Nlog N)。
    pub fn snapshot(&self) -> Vec<Entry> {
        // 复制然后排序以反映堆序；只读场景下更直观。
        let mut v = self.data.clone();
        v.sort_by(|a, b| {
            b.score.cmp(&a.score).then(a.arrived_tick.cmp(&b.arrived_tick))
        });
        v
    }

    // ---- 内部 ----

    fn swap(&mut self, a: usize, b: usize) {
        self.data.swap(a, b);
        self.data[a].heap_idx = a;
        self.data[b].heap_idx = b;
        self.pos.insert(self.data[a].id.clone(), a);
        self.pos.insert(self.data[b].id.clone(), b);
    }

    fn higher(a: &Entry, b: &Entry) -> bool {
        // true 表示 a 应该在 b 之上
        a.score > b.score || (a.score == b.score && a.arrived_tick < b.arrived_tick)
    }

    fn sift_up(&mut self, mut i: usize) {
        while i > 0 {
            let parent = (i - 1) / 2;
            if Self::higher(&self.data[i], &self.data[parent]) {
                self.swap(i, parent);
                i = parent;
            } else {
                break;
            }
        }
    }

    fn sift_down(&mut self, mut i: usize) {
        let n = self.data.len();
        loop {
            let l = 2 * i + 1;
            let r = 2 * i + 2;
            let mut best = i;
            if l < n && Self::higher(&self.data[l], &self.data[best]) { best = l; }
            if r < n && Self::higher(&self.data[r], &self.data[best]) { best = r; }
            if best == i { break; }
            self.swap(i, best);
            i = best;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mk(id: &str, score: u32, t: u64) -> Entry {
        Entry { id: id.into(), score, arrived_tick: t, heap_idx: INVALID }
    }
    #[test]
    fn push_pop_order() {
        let mut h = IndexedMaxHeap::default();
        h.push(mk("a", 5, 0));
        h.push(mk("b", 10, 1));
        h.push(mk("c", 7, 2));
        assert_eq!(h.pop().unwrap().id, "b");
        assert_eq!(h.pop().unwrap().id, "c");
        assert_eq!(h.pop().unwrap().id, "a");
    }
    #[test]
    fn remove_maintains_heap() {
        let mut h = IndexedMaxHeap::default();
        h.push(mk("a", 5, 0));
        h.push(mk("b", 8, 1));
        h.push(mk("c", 7, 2));
        h.remove("b").unwrap();
        assert_eq!(h.peek().unwrap().id, "c");
        let snap: Vec<_> = h.snapshot().into_iter().map(|e| e.id).collect();
        assert_eq!(snap, vec!["c", "a"]);
    }
}
