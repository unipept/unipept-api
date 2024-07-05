use std::{
    collections::HashMap,
    hash::Hash,
    ops::Deref
};

#[derive(Debug)]
pub struct FrequencyTable<T: Hash + Eq + Clone>(HashMap<T, usize>);

impl<T: Hash + Eq + Clone> FrequencyTable<T> {
    pub fn from_data(data: &[T]) -> Self {
        let mut frequency_table = HashMap::new();

        for item in data {
            *frequency_table.entry(item.clone()).or_insert(0) += 1;
        }

        FrequencyTable(frequency_table)
    }

    pub fn from_counts(data: HashMap<T, usize>) -> Self {
        FrequencyTable(data)
    }
}

impl<T: Hash + Eq + Clone> Deref for FrequencyTable<T> {
    type Target = HashMap<T, usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
