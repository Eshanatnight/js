use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

#[derive(Debug, Clone)]
pub enum NodeKind {
    Object(usize),
    Array(usize),
    String(String),
    Number(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub key: Option<String>,
    pub kind: NodeKind,
    pub depth: usize,
    pub expanded: bool,
    pub is_array_element: bool,
    pub children: Vec<usize>,
}

impl TreeNode {
    pub const fn is_expandable(&self) -> bool {
        match &self.kind {
            NodeKind::Object(n) | NodeKind::Array(n) => *n > 0,
            _ => false,
        }
    }
}

pub struct JsonTree {
    pub nodes: Vec<TreeNode>,
    pub root: usize,
}

impl JsonTree {
    pub fn from_str(json: &str) -> Result<Self, serde_json::Error> {
        let mut nodes = Vec::with_capacity(json.len() / 32);
        let mut de = serde_json::Deserializer::from_str(json);
        let root = TreeSeed::new(&mut nodes, None, 0, false).deserialize(&mut de)?;
        de.end()?;
        Ok(Self { nodes, root })
    }

    pub fn visible_lines(&self) -> Vec<usize> {
        let mut lines = Vec::new();
        self.collect_visible(self.root, &mut lines);
        lines
    }

    fn collect_visible(&self, idx: usize, lines: &mut Vec<usize>) {
        lines.push(idx);
        let node = &self.nodes[idx];
        if node.expanded {
            for &child in &node.children {
                self.collect_visible(child, lines);
            }
        }
    }

    pub fn toggle(&mut self, idx: usize) {
        if self.nodes[idx].is_expandable() {
            self.nodes[idx].expanded = !self.nodes[idx].expanded;
        }
    }

    pub fn expand_all(&mut self) {
        for node in &mut self.nodes {
            if node.is_expandable() {
                node.expanded = true;
            }
        }
    }

    pub fn collapse_all(&mut self) {
        for node in &mut self.nodes {
            node.expanded = false;
        }
        if self.nodes[self.root].is_expandable() {
            self.nodes[self.root].expanded = true;
        }
    }

    pub fn get_path(&self, target: usize) -> String {
        let mut parts = Vec::new();
        if self.build_path(self.root, target, &mut parts) {
            if parts.is_empty() {
                "$".to_string()
            } else {
                format!("${}", parts.join(""))
            }
        } else {
            "$".to_string()
        }
    }

    fn build_path(&self, current: usize, target: usize, parts: &mut Vec<String>) -> bool {
        if current == target {
            return true;
        }
        let node = &self.nodes[current];
        for &child in &node.children {
            let child_node = &self.nodes[child];
            let part = if child_node.is_array_element {
                child_node
                    .key
                    .as_ref()
                    .map(|k| format!("[{k}]"))
                    .unwrap_or_default()
            } else {
                child_node
                    .key
                    .as_ref()
                    .map(|k| format!(".{k}"))
                    .unwrap_or_default()
            };
            parts.push(part);
            if self.build_path(child, target, parts) {
                return true;
            }
            parts.pop();
        }
        false
    }

    pub fn node_matches(&self, idx: usize, query: &str) -> bool {
        let node = &self.nodes[idx];
        let q = query.to_lowercase();

        if let Some(k) = &node.key
            && k.to_lowercase().contains(&q)
        {
            return true;
        }

        match &node.kind {
            NodeKind::String(s) => s.to_lowercase().contains(&q),
            NodeKind::Number(n) => n.contains(query),
            NodeKind::Bool(b) => b.to_string().contains(&q),
            NodeKind::Null => "null".contains(&q),
            _ => false,
        }
    }

    pub fn expand_to_depth(&mut self, max_depth: usize) {
        for node in &mut self.nodes {
            if node.is_expandable() {
                node.expanded = node.depth < max_depth;
            }
        }
    }

    pub fn node_to_json(&self, idx: usize) -> serde_json::Value {
        let node = &self.nodes[idx];
        match &node.kind {
            NodeKind::Null => serde_json::Value::Null,
            NodeKind::Bool(b) => serde_json::Value::Bool(*b),
            NodeKind::Number(n) => n.parse::<serde_json::Number>().map_or_else(
                |_| serde_json::Value::String(n.clone()),
                serde_json::Value::Number,
            ),
            NodeKind::String(s) => serde_json::Value::String(s.clone()),
            NodeKind::Object(_) => {
                let map = node
                    .children
                    .iter()
                    .map(|&c| {
                        let key = self.nodes[c].key.clone().unwrap_or_default();
                        (key, self.node_to_json(c))
                    })
                    .collect();
                serde_json::Value::Object(map)
            }
            NodeKind::Array(_) => {
                let arr = node
                    .children
                    .iter()
                    .map(|&c| self.node_to_json(c))
                    .collect();
                serde_json::Value::Array(arr)
            }
        }
    }

    pub fn node_value_string(&self, idx: usize) -> String {
        match &self.nodes[idx].kind {
            NodeKind::String(s) => s.clone(),
            NodeKind::Number(n) => n.clone(),
            NodeKind::Bool(b) => b.to_string(),
            NodeKind::Null => "null".to_string(),
            NodeKind::Object(_) | NodeKind::Array(_) => {
                serde_json::to_string_pretty(&self.node_to_json(idx)).unwrap_or_default()
            }
        }
    }
}

struct TreeSeed<'nodes> {
    nodes: &'nodes mut Vec<TreeNode>,
    key: Option<String>,
    depth: usize,
    is_array_element: bool,
}

impl<'nodes> TreeSeed<'nodes> {
    const fn new(
        nodes: &'nodes mut Vec<TreeNode>,
        key: Option<String>,
        depth: usize,
        is_array_element: bool,
    ) -> Self {
        Self {
            nodes,
            key,
            depth,
            is_array_element,
        }
    }

    fn push_leaf(self, kind: NodeKind) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(TreeNode {
            key: self.key,
            kind,
            depth: self.depth,
            expanded: false,
            is_array_element: self.is_array_element,
            children: Vec::new(),
        });
        idx
    }
}

impl<'de> DeserializeSeed<'de> for TreeSeed<'_> {
    type Value = usize;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<usize, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for TreeSeed<'_> {
    type Value = usize;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<usize, E> {
        Ok(self.push_leaf(NodeKind::Bool(v)))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<usize, E> {
        Ok(self.push_leaf(NodeKind::Number(v.to_string())))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<usize, E> {
        Ok(self.push_leaf(NodeKind::Number(v.to_string())))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<usize, E> {
        Ok(self.push_leaf(NodeKind::Number(v.to_string())))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<usize, E> {
        Ok(self.push_leaf(NodeKind::String(v.to_owned())))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<usize, E> {
        Ok(self.push_leaf(NodeKind::String(v)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<usize, E> {
        Ok(self.push_leaf(NodeKind::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<usize, A::Error> {
        let nodes = self.nodes;
        let depth = self.depth;
        let idx = nodes.len();
        nodes.push(TreeNode {
            key: self.key,
            kind: NodeKind::Array(0),
            depth,
            expanded: depth < 1,
            is_array_element: self.is_array_element,
            children: Vec::new(),
        });
        let mut children = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        let mut i = 0_usize;
        while let Some(child) = seq.next_element_seed(TreeSeed::new(
            &mut *nodes,
            Some(i.to_string()),
            depth + 1,
            true,
        ))? {
            children.push(child);
            i += 1;
        }
        let count = children.len();
        nodes[idx].kind = NodeKind::Array(count);
        nodes[idx].children = children;
        Ok(idx)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<usize, A::Error> {
        let nodes = self.nodes;
        let depth = self.depth;
        let idx = nodes.len();
        nodes.push(TreeNode {
            key: self.key,
            kind: NodeKind::Object(0),
            depth,
            expanded: depth < 1,
            is_array_element: self.is_array_element,
            children: Vec::new(),
        });
        let mut children = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let child =
                map.next_value_seed(TreeSeed::new(&mut *nodes, Some(key), depth + 1, false))?;
            children.push(child);
        }
        let count = children.len();
        nodes[idx].kind = NodeKind::Object(count);
        nodes[idx].children = children;
        Ok(idx)
    }
}
