#[derive(Clone)]
pub enum DuplicateMap {
    Single(String),
    List(Vec<String>),
}

impl DuplicateMap {
    pub fn as_single(&self) -> Option<&String> {
        match self {
            DuplicateMap::Single(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<String>> {
        match self {
            DuplicateMap::List(values) => Some(values),
            _ => None,
        }
    }
    
    pub fn as_slice(&self) -> Vec<&String> {
        match self {
            DuplicateMap::Single(value) => vec![value],
            DuplicateMap::List(values) => values.iter().collect(),
        }
    }
}

#[derive(Clone)]
pub struct Map<T> {
    params: Vec<(String, T)>,
}

impl<T> Map<T> {
    pub fn has(&self, index: &str) -> bool {
        return self.params.iter().any(|key| key.0 == index);
    }

    pub fn get(&self, index: &str) -> Option<&T> {
        return self
            .params
            .iter()
            .find(|x| x.0.as_str() == index)
            .map(|value| &value.1);
    }
}

impl Map<DuplicateMap> {
    pub fn add(&mut self, key: &str, value: String) {
        for entry in self.params.iter_mut() {
            if &entry.0 == key {
                match &mut entry.1 {
                    DuplicateMap::Single(original_value) => {
                        entry.1 = DuplicateMap::List(vec![original_value.clone(), value]);
                        return;
                    }
                    DuplicateMap::List(items) => {
                        items.push(value);
                        return;
                    }
                }
            }
        }
        self.params
            .push((key.to_owned(), DuplicateMap::Single(value)))
    }
    
    pub fn get_require_single(&self, index: &str) -> Result<Option<&String>, String> {
        match self.get(index) {
            Some(DuplicateMap::Single(value)) => Ok(Some(value)),
            Some(DuplicateMap::List(_)) => Err(format!(
                "Expected single value for key '{}', but found multiple values.",
                index
            )),
            _ => Ok(None),
        }
    }
    
    pub fn get_single(&self, index: &str) -> Option<&String> {
        match self.get(index) {
            Some(DuplicateMap::Single(value)) => Some(value),
            _ => None,
        }
    }
    
    pub fn add_require_single(&mut self, key: &str, value: String) -> Result<(), String> {
        for entry in self.params.iter_mut() {
            if &entry.0 == key {
                match &entry.1 {
                    DuplicateMap::Single(_) => {
                        return Err(format!(
                            "Key '{}' already has a single value, cannot add another.",
                            key
                        ));
                    }
                    DuplicateMap::List(_) => {
                        return Err(format!(
                            "Key '{}' already has multiple values, cannot add a single value.",
                            key
                        ));
                    }
                }
            }
        }
        self.params
            .push((key.to_owned(), DuplicateMap::Single(value)));
        Ok(())
    }
}

impl Map<String> {
    pub fn add(&mut self, key: &str, value: String) {
        self.params.push((key.to_owned(), value))
    }
}

impl<T> Default for Map<T> {
    fn default() -> Self {
        Self {
            params: Default::default(),
        }
    }
}
