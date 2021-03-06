use crate::{
    external_error, type_as_string,
    value::{deep_copy_value, value_is_callable},
    value_is_immutable,
    value_iterator::ValueIteratorOutput as Output,
    RuntimeResult, Value, ValueHashMap, ValueIterator, ValueMap, Vm,
};

pub fn make_module() -> ValueMap {
    use Value::*;

    let mut result = ValueMap::new();

    result.add_fn("clear", |vm, args| match vm.get_args(args) {
        [Map(m)] => {
            m.data_mut().clear();
            Ok(Empty)
        }
        _ => external_error!("map.clear: Expected map as argument"),
    });

    result.add_fn("contains_key", |vm, args| match vm.get_args(args) {
        [Map(m), key] => Ok(Bool(m.data().contains_key(key))),
        [other_a, other_b, ..] => external_error!(
            "map.contains_key: Expected map and key as arguments, found '{}' and '{}'",
            type_as_string(other_a),
            type_as_string(other_b)
        ),
        _ => external_error!("map.contains_key: Expected map and key as arguments"),
    });

    result.add_fn("copy", |vm, args| match vm.get_args(args) {
        [Map(m)] => Ok(Map(ValueMap::with_data(m.data().clone()))),
        _ => external_error!("map.copy: Expected map as argument"),
    });

    result.add_fn("deep_copy", |vm, args| match vm.get_args(args) {
        [value @ Map(_)] => Ok(deep_copy_value(value)),
        _ => external_error!("map.deep_copy: Expected map as argument"),
    });

    result.add_fn("get", |vm, args| match vm.get_args(args) {
        [Map(m), key] => match m.data().get(key) {
            Some(value) => Ok(value.clone()),
            None => Ok(Empty),
        },
        [other_a, other_b, ..] => external_error!(
            "map.get: Expected map and key as arguments, found '{}' and '{}'",
            type_as_string(other_a),
            type_as_string(other_b)
        ),
        _ => external_error!("map.get: Expected map and key as arguments"),
    });

    result.add_fn("get_index", |vm, args| match vm.get_args(args) {
        [Map(m), Number(n)] => {
            if *n < 0.0 {
                return external_error!("map.get_index: Negative indices aren't allowed");
            }
            match m.data().get_index(n.into()) {
                Some((key, value)) => Ok(Tuple(vec![key.clone(), value.clone()].into())),
                None => Ok(Empty),
            }
        }
        _ => external_error!("map.get_index: Expected map and index as arguments"),
    });

    result.add_fn("insert", |vm, args| match vm.get_args(args) {
        [Map(m), key] if value_is_immutable(key) => match m.data_mut().insert(key.clone(), Empty) {
            Some(old_value) => Ok(old_value),
            None => Ok(Empty),
        },
        [Map(m), key, value] if value_is_immutable(key) => {
            match m.data_mut().insert(key.clone(), value.clone()) {
                Some(old_value) => Ok(old_value),
                None => Ok(Empty),
            }
        }
        [other_a, other_b, ..] => external_error!(
            "map.insert: Expected map and key as arguments, found '{}' and '{}'",
            type_as_string(other_a),
            type_as_string(other_b)
        ),
        _ => external_error!("map.insert: Expected map and key as arguments"),
    });

    result.add_fn("is_empty", |vm, args| match vm.get_args(args) {
        [Map(m)] => Ok(Bool(m.data().is_empty())),
        [other, ..] => external_error!(
            "map.is_empty: Expected map as argument, found '{}'",
            type_as_string(other),
        ),
        _ => external_error!("map.contains_key: Expected map and key as arguments"),
    });

    result.add_fn("iter", |vm, args| match vm.get_args(args) {
        [Map(m)] => Ok(Iterator(ValueIterator::with_map(m.clone()))),
        [other, ..] => external_error!(
            "map.iter: Expected map as argument, found '{}'",
            type_as_string(other),
        ),
        _ => external_error!("map.iter: Expected map as argument"),
    });

    result.add_fn("keys", |vm, args| match vm.get_args(args) {
        [Map(m)] => {
            let mut iter = ValueIterator::with_map(m.clone()).map(|output| match output {
                Ok(Output::ValuePair(key, _)) => Ok(Output::Value(key)),
                Ok(_) => unreachable!(),
                Err(e) => Err(e),
            });

            Ok(Iterator(ValueIterator::make_external(move || iter.next())))
        }
        [other, ..] => external_error!(
            "map.keys: Expected map as argument, found '{}'",
            type_as_string(other),
        ),
        _ => external_error!("map.keys: Expected map as argument"),
    });

    result.add_fn("remove", |vm, args| match vm.get_args(args) {
        [Map(m), key] if value_is_immutable(key) => match m.data_mut().shift_remove(key) {
            Some(old_value) => Ok(old_value),
            None => Ok(Empty),
        },
        [other_a, other_b, ..] => external_error!(
            "map.remove: Expected map and key as arguments, found '{}' and '{}'",
            type_as_string(other_a),
            type_as_string(other_b)
        ),
        _ => external_error!("map.remove: Expected map and key as arguments"),
    });

    result.add_fn("sort", |vm, args| match vm.get_args(args) {
        [Map(m)] => {
            m.data_mut().sort_keys();
            Ok(Empty)
        }
        [Map(l), f] if value_is_callable(f) => {
            let m = l.clone();
            let f = f.clone();
            let mut vm = vm.spawn_shared_vm();
            let mut error = None;

            let mut get_sort_key = |cache: &mut ValueHashMap, key: &Value, value: &Value| {
                let value = match vm.run_function(f.clone(), &[key.clone(), value.clone()]) {
                    Ok(value) => value,
                    Err(e) => {
                        error.get_or_insert(Err(e.with_prefix("map.sort")));
                        Empty
                    }
                };
                cache.insert(key.clone(), value.clone());
                value
            };

            let mut cache = ValueHashMap::with_capacity(m.len());
            m.data_mut().sort_by(|key_a, value_a, key_b, value_b| {
                let value_a = match cache.get(key_a) {
                    Some(value) => value.clone(),
                    None => get_sort_key(&mut cache, key_a, value_a),
                };
                let value_b = match cache.get(key_b) {
                    Some(value) => value.clone(),
                    None => get_sort_key(&mut cache, key_b, value_b),
                };
                value_a.cmp(&value_b)
            });

            if let Some(error) = error {
                error
            } else {
                Ok(Empty)
            }
        }
        _ => external_error!("map.sort: Expected map as argument"),
    });

    result.add_fn("size", |vm, args| match vm.get_args(args) {
        [Map(m)] => Ok(Number(m.data().len().into())),
        [other, ..] => external_error!(
            "map.size: Expected map as argument, found '{}'",
            type_as_string(other),
        ),
        _ => external_error!("map.contains_key: Expected map and key as arguments"),
    });

    result.add_fn("update", |vm, args| match vm.get_args(args) {
        [Map(m), key, f] if value_is_immutable(key) && value_is_callable(f) => do_map_update(
            m.clone(),
            key.clone(),
            Empty,
            f.clone(),
            vm.spawn_shared_vm(),
        ),
        [Map(m), key, default, f] if value_is_immutable(key) && value_is_callable(f) => {
            do_map_update(
                m.clone(),
                key.clone(),
                default.clone(),
                f.clone(),
                vm.spawn_shared_vm(),
            )
        }
        _ => external_error!("map.update: Expected map, key, and function as arguments"),
    });

    result.add_fn("values", |vm, args| match vm.get_args(args) {
        [Map(m)] => {
            let mut iter = ValueIterator::with_map(m.clone()).map(|output| match output {
                Ok(Output::ValuePair(_, value)) => Ok(Output::Value(value)),
                Ok(_) => unreachable!(),
                Err(e) => Err(e),
            });

            Ok(Iterator(ValueIterator::make_external(move || iter.next())))
        }
        [other, ..] => external_error!(
            "map.values: Expected map as argument, found '{}'",
            type_as_string(other),
        ),
        _ => external_error!("map.values: Expected map as argument"),
    });

    result
}

fn do_map_update(map: ValueMap, key: Value, default: Value, f: Value, mut vm: Vm) -> RuntimeResult {
    let mut vm = vm.spawn_shared_vm();
    if !map.data().contains_key(&key) {
        map.data_mut().insert(key.clone(), default);
    }
    let value = map.data().get(&key).cloned().unwrap();
    match vm.run_function(f, &[value]) {
        Ok(new_value) => {
            map.data_mut().insert(key, new_value.clone());
            Ok(new_value)
        }
        Err(error) => Err(error.with_prefix("map.update")),
    }
}
