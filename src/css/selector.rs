use crate::dom::Element;

#[derive(Debug, PartialEq, Clone)]
pub enum Selector {
    Simple(SimpleSelector),
}

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleSelector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

impl SimpleSelector {
    pub fn specificity(&self) -> (usize, usize, usize) {
        let id = self.id.as_ref().map_or(0, |_| 1);
        let classes = self.classes.len();
        let tag = self.tag.as_ref().map_or(0, |_| 1);
        (id, classes, tag)
    }
}

pub fn matches(selector: &SimpleSelector, element: &Element) -> bool {
    // Check tag
    if let Some(ref tag) = selector.tag {
        if &element.tag != tag {
            return false;
        }
    }

    // Check id
    if let Some(ref id) = selector.id {
        match element.attrs.get("id") {
            Some(val) if val == id => {}
            _ => return false,
        }
    }

    // Check classes
    if !selector.classes.is_empty() {
        let element_classes = element_classes(element);
        for class in &selector.classes {
            if !element_classes.contains(class) {
                return false;
            }
        }
    }

    true
}

fn element_classes(element: &Element) -> Vec<String> {
    match element.attrs.get("class") {
        Some(class_str) => class_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::dom::Element;

    fn make_element(tag: &str, attrs: HashMap<String, String>) -> Element {
        Element::new(tag, attrs, vec![])
    }

    #[test]
    fn test_selector_tag_match() {
        let selector = SimpleSelector {
            tag: Some("div".to_string()),
            id: None,
            classes: vec![],
        };
        let el = make_element("div", HashMap::new());
        assert!(matches(&selector, &el));
    }

    #[test]
    fn test_selector_tag_no_match() {
        let selector = SimpleSelector {
            tag: Some("div".to_string()),
            id: None,
            classes: vec![],
        };
        let el = make_element("p", HashMap::new());
        assert!(!matches(&selector, &el));
    }

    #[test]
    fn test_selector_id_match() {
        let selector = SimpleSelector {
            tag: None,
            id: Some("main".to_string()),
            classes: vec![],
        };
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "main".to_string());
        let el = make_element("div", attrs);
        assert!(matches(&selector, &el));
    }

    #[test]
    fn test_selector_class_match() {
        let selector = SimpleSelector {
            tag: None,
            id: None,
            classes: vec!["foo".to_string()],
        };
        let mut attrs = HashMap::new();
        attrs.insert("class".to_string(), "foo bar".to_string());
        let el = make_element("div", attrs);
        assert!(matches(&selector, &el));
    }

    #[test]
    fn test_selector_specificity() {
        let tag_sel = SimpleSelector {
            tag: Some("div".to_string()),
            id: None,
            classes: vec![],
        };
        assert_eq!(tag_sel.specificity(), (0, 0, 1));

        let class_sel = SimpleSelector {
            tag: None,
            id: None,
            classes: vec!["foo".to_string()],
        };
        assert_eq!(class_sel.specificity(), (0, 1, 0));

        let id_sel = SimpleSelector {
            tag: None,
            id: Some("main".to_string()),
            classes: vec![],
        };
        assert_eq!(id_sel.specificity(), (1, 0, 0));

        let combined = SimpleSelector {
            tag: Some("div".to_string()),
            id: Some("main".to_string()),
            classes: vec!["container".to_string()],
        };
        assert_eq!(combined.specificity(), (1, 1, 1));
    }

    #[test]
    fn test_selector_combined() {
        let selector = SimpleSelector {
            tag: Some("div".to_string()),
            id: Some("main".to_string()),
            classes: vec!["container".to_string()],
        };
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "main".to_string());
        attrs.insert("class".to_string(), "container active".to_string());
        let el = make_element("div", attrs);
        assert!(matches(&selector, &el));
    }

    #[test]
    fn test_selector_combined_no_match_wrong_tag() {
        let selector = SimpleSelector {
            tag: Some("div".to_string()),
            id: Some("main".to_string()),
            classes: vec!["container".to_string()],
        };
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "main".to_string());
        attrs.insert("class".to_string(), "container".to_string());
        let el = make_element("span", attrs);
        assert!(!matches(&selector, &el));
    }

    #[test]
    fn test_selector_combined_no_match_wrong_id() {
        let selector = SimpleSelector {
            tag: Some("div".to_string()),
            id: Some("main".to_string()),
            classes: vec!["container".to_string()],
        };
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "other".to_string());
        attrs.insert("class".to_string(), "container".to_string());
        let el = make_element("div", attrs);
        assert!(!matches(&selector, &el));
    }

    #[test]
    fn test_selector_combined_no_match_missing_class() {
        let selector = SimpleSelector {
            tag: Some("div".to_string()),
            id: Some("main".to_string()),
            classes: vec!["container".to_string()],
        };
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "main".to_string());
        attrs.insert("class".to_string(), "other".to_string());
        let el = make_element("div", attrs);
        assert!(!matches(&selector, &el));
    }
}
