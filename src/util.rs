use std::fs;

use pyo3::{IntoPy, PyObject, Python};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::{Number, Result, Value};

use crate::config::Config;

pub fn load_config(file_path: &str) -> Result<Config> {
    let config_data = fs::read_to_string(file_path).unwrap();
    let config: Config = serde_json::from_str(&config_data).unwrap();
    Ok(config)
}


pub fn load_py_config(config: PyObject) -> Result<Config> {
    Python::with_gil(|py| {
        let value: Value = py_to_json(py, config);
        let config: Config = serde_json::from_value(value).unwrap();
        Ok(config)
    })
}

pub fn json_to_py(py: Python, value: &Value) -> PyObject {
    match value {
        Value::Null => py.None(),
        Value::Bool(b) => b.into_py(py),
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                i.into_py(py)
            } else if let Some(f) = num.as_f64() {
                f.into_py(py)
            } else {
                py.None()
            }
        }
        Value::String(s) => s.into_py(py),
        Value::Array(arr) => {
            let py_list = PyList::new(py, arr.iter().map(|v| json_to_py(py, v)));
            py_list.ok().into_py(py)
        }
        Value::Object(obj) => {
            let py_dict = PyDict::new(py);
            for (k, v) in obj {
                py_dict.set_item(k, json_to_py(py, v)).unwrap();
            }
            py_dict.into_py(py)
        }
    }
}


pub fn py_to_json(py: Python, obj: PyObject) -> Value {
    if obj.is(&py.None()) {
        return Value::Null;
    }

    if let Some(py_bool) = obj.downcast_bound::<pyo3::types::PyBool>(py).ok() {
        return Value::Bool(py_bool.is_true());
    }

    if let Some(py_int) = obj.downcast_bound::<pyo3::types::PyLong>(py).ok() {
        return Value::Number(py_int.extract::<i64>().unwrap().into());
    }

    if let Some(py_float) = obj.downcast_bound::<pyo3::types::PyFloat>(py).ok() {
        return Value::Number(Number::from_f64(py_float.extract::<f64>().unwrap()).unwrap());
    }

    if let Some(py_str) = obj.downcast_bound::<pyo3::types::PyString>(py).ok() {
        return Value::String(py_str.to_str().unwrap().to_string());
    }

    if let Some(py_list) = obj.downcast_bound::<PyList>(py).ok() {
        let arr: Vec<Value> = py_list
            .iter()
            .map(|item| py_to_json(py, item.into()))
            .collect();
        return Value::Array(arr);
    }

    if let Some(py_dict) = obj.downcast_bound::<PyDict>(py).ok() {
        let mut obj_map = serde_json::Map::new();
        for (key, value) in py_dict {
            let key_str = key.to_string();
            obj_map.insert(key_str, py_to_json(py, value.into()));
        }
        return Value::Object(obj_map);
    }

    Value::Null
}

