use bazel_lsp::target_trie::{RuleInfo, TargetTrie};

#[test]
fn test_trie_insert_and_search() {
    let mut trie = TargetTrie::new();
    let rule = RuleInfo {
        name: "test_rule".to_string(),
        full_build_path: "//a/b:c".to_string(),
    };
    trie.insert_target("//a/b:c", rule);

    let results = trie.starts_with("//a/b:c");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0][0].name, "test_rule");
}

#[test]
fn test_trie_starts_with() {
    let mut trie = TargetTrie::new();
    let rule1 = RuleInfo {
        name: "rule1".to_string(),
        full_build_path: "//a/b:c".to_string(),
    };
    let rule2 = RuleInfo {
        name: "rule2".to_string(),
        full_build_path: "//a/b:d".to_string(),
    };
    let rule3 = RuleInfo {
        name: "rule3".to_string(),
        full_build_path: "//a/c:e".to_string(),
    };

    trie.insert_target("//a/b:c", rule1);
    trie.insert_target("//a/b:d", rule2);
    trie.insert_target("//a/c:e", rule3);

    let results = trie.starts_with("//a/b");
    assert_eq!(results.len(), 2);

    let results = trie.starts_with("//a");
    assert_eq!(results.len(), 3);

    let results = trie.starts_with("//b");
    assert_eq!(results.len(), 0);
}

#[test]
fn test_trie_with_package_path() {
    let mut trie = TargetTrie::new();
    let rule = RuleInfo {
        name: "main".to_string(),
        full_build_path: "//src:main".to_string(),
    };
    trie.insert_target("//src:main", rule);

    let results = trie.starts_with("//src");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0][0].name, "main");
}
