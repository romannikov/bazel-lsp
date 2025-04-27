use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RuleInfo {
    pub name: String,
}

#[derive(Debug)]
pub struct TrieNode {
    pub char: char,
    pub is_end: bool,
    pub is_package_end: bool,
    pub rules: Vec<RuleInfo>,
    pub children: HashMap<char, TrieNode>,
}

impl TrieNode {
    pub fn new(char: char) -> Self {
        Self {
            char,
            is_end: false,
            is_package_end: false,
            rules: Vec::new(),
            children: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct TargetTrie {
    root: TrieNode,
}

impl TargetTrie {
    pub fn new() -> Self {
        Self {
            root: TrieNode {
                char: '\0',
                is_end: false,
                is_package_end: false,
                rules: Vec::new(),
                children: HashMap::new(),
            },
        }
    }

    pub fn insert_target(&mut self, path: &str, rule: RuleInfo) {
        let mut current = &mut self.root;

        let (package_path, rule_name) = if path.contains(':') {
            let parts: Vec<&str> = path.split(':').collect();
            (parts[0], parts[1])
        } else {
            ("", path)
        };

        if !package_path.is_empty() {
            let parts: Vec<&str> = package_path.split('/').collect();
            for (i, part) in parts.iter().enumerate() {
                for c in part.chars() {
                    current = current
                        .children
                        .entry(c)
                        .or_insert_with(|| TrieNode::new(c));
                }

                if i < parts.len() - 1 {
                    current.is_package_end = true;
                }
            }
        }

        for c in rule_name.chars() {
            current = current
                .children
                .entry(c)
                .or_insert_with(|| TrieNode::new(c));
        }

        current.is_end = true;
        current.rules.push(rule);
    }

    pub fn starts_with(&self, prefix: &str) -> Vec<&Vec<RuleInfo>> {
        let mut result = Vec::new();
        let mut current = &self.root;

        if prefix.is_empty() {
            let mut stack = vec![current];
            while let Some(node) = stack.pop() {
                if node.is_end && !node.rules.is_empty() {
                    result.push(&node.rules);
                }
                for child in node.children.values() {
                    stack.push(child);
                }
            }
            return result;
        }

        let (package_path, rule_prefix) = if prefix.contains(':') {
            let parts: Vec<&str> = prefix.split(':').collect();
            (parts[0], parts[1])
        } else {
            (prefix, "")
        };

        let parts: Vec<&str> = package_path.split('/').collect();
        for part in parts.iter() {
            for c in part.chars() {
                match current.children.get(&c) {
                    Some(node) => current = node,
                    None => return result,
                }
            }
        }

        if !rule_prefix.is_empty() {
            for c in rule_prefix.chars() {
                match current.children.get(&c) {
                    Some(node) => current = node,
                    None => return result,
                }
            }
        }

        let mut stack = vec![current];
        while let Some(node) = stack.pop() {
            if node.is_end && !node.rules.is_empty() {
                result.push(&node.rules);
            }
            for child in node.children.values() {
                stack.push(child);
            }
        }

        result
    }
}

impl Default for TargetTrie {
    fn default() -> Self {
        Self::new()
    }
}
