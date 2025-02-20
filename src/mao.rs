use std::{collections::HashMap, sync::Arc};

pub mod automaton;
pub mod mao_action;
pub mod mao_core;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct DataStorageType(Vec<u8>);
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct DataKey(Arc<str>);
#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub struct Data {
    data: HashMap<DataKey, DataStorageType>,
}

pub trait DataContainer {
    fn data(&self) -> &Data;
    fn data_mut(&mut self) -> &mut Data;
    fn get_or_insert_rule_data(&mut self, rule_name: DataKey) -> &DataStorageType {
        if self.data().data.contains_key(&rule_name) {
            return self.data().data.get(&rule_name).unwrap();
        }
        self.data_mut()
            .data
            .insert(DataKey::clone(&rule_name), DataStorageType::default());
        self.data().data.get(&rule_name).unwrap()
    }

    fn mutate_rule_data(&mut self, rule_name: DataKey) -> &mut DataStorageType {
        if self.data().data.contains_key(&rule_name) {
            return self.data_mut().data.get_mut(&rule_name).unwrap();
        }
        self.data_mut()
            .data
            .insert(DataKey::clone(&rule_name), DataStorageType::default());
        self.data_mut().data.get_mut(&rule_name).unwrap()
    }
    fn get_rule_data(&self, rule_name: DataKey) -> Option<&DataStorageType> {
        self.data().data.get(&rule_name)
    }
}
