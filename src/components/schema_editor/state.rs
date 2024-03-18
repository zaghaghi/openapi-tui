use std::{
  collections::BTreeMap,
  sync::{Arc, RwLock},
};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use oas3::{
  spec::{RefError, SchemaType},
  Schema,
};
use tui_prompts::{State, TextState};

use crate::{action::Action, pages::home::State as GlobalState, tui::EventResponse};

pub enum SchemaEditorPromptType {
  String,
  Password,
  Int,
  Float,
  Bool,
}

pub struct SchemaEditorPageState<'a> {
  pub prop_name: String,

  pub inside: bool,
  pub selected: usize,
  pub fields: Vec<String>,
  pub prompt_states: BTreeMap<String, (SchemaEditorPromptType, RwLock<TextState<'a>>)>,
  pub children: BTreeMap<String, SchemaEditorPageState<'a>>,
}

#[derive(Default)]
pub struct SchemaEditorState<'a> {
  pub root: Option<SchemaEditorPageState<'a>>,
  pub inside: bool,
}

impl SchemaEditorPageState<'_> {
  pub fn new(prop_name: String, schema: &Schema, global_state: Arc<RwLock<GlobalState>>) -> Result<Self, RefError> {
    let mut prompt_states = BTreeMap::new();
    let mut children = BTreeMap::new();
    let mut fields = Vec::with_capacity(schema.properties.len());

    for (key, value) in &schema.properties {
      let value = value.resolve(&global_state.read().unwrap().openapi_spec)?;

      fields.push(key.clone());
      match value.schema_type {
        Some(oas3::spec::SchemaType::Object) => {
          children.insert(key.clone(), SchemaEditorPageState::new(key.clone(), &value, global_state.clone())?);
        },
        _ => {
          let type_ = value.schema_type.unwrap_or(SchemaType::String);
          let format = value.format.unwrap_or(String::from("string"));
          let format = match (type_, &format as &str) {
            (SchemaType::String, "password") => SchemaEditorPromptType::Password,
            (SchemaType::String, "int32") => SchemaEditorPromptType::Int,
            (SchemaType::String, "int64") => SchemaEditorPromptType::Int,
            (SchemaType::String, _) => SchemaEditorPromptType::String,

            (SchemaType::Integer, _) => SchemaEditorPromptType::Int,
            (SchemaType::Number, _) => SchemaEditorPromptType::Float,

            (SchemaType::Boolean, _) => SchemaEditorPromptType::Float,

            (SchemaType::Array, _) => {
              log::warn!("Array type on schema editor");
              SchemaEditorPromptType::String
            },

            _ => {
              log::warn!("[SchemaEditor] Cannot match type and format to create a prompt ({type_:?}, {format:?})");
              SchemaEditorPromptType::String
            },
          };
          let state = RwLock::new(TextState::new());
          prompt_states.insert(key.clone(), (format, state));
        },
      }
    }

    fields.sort();

    Ok(Self { prop_name, inside: false, selected: 0, fields, prompt_states, children })
  }

  pub fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    if self.inside {
      let Some(field) = self.fields.get(self.selected) else {
        return Ok(None);
      };
      if let Some(prompt_state) = self.prompt_states.get(field) {
        if matches!(key.code, KeyCode::Esc) {
          self.inside = false;
          return Ok(Some(EventResponse::Stop(Action::Render)));
        }
        if matches!(key.code, KeyCode::Enter) {
          self.down();
        } else {
          prompt_state.1.write().unwrap().handle_key_event(key);
        }

        return Ok(Some(EventResponse::Stop(Action::Render)));
      } else if let Some(children) = self.children.get_mut(field) {
        let resp = children.handle_key_events(key);
        if matches!(resp, Ok(Some(EventResponse::Stop(Action::Back)))) {
          self.inside = false;
          return Ok(Some(EventResponse::Stop(Action::Render)));
        }

        return resp;
      }
    }

    if matches!(key.code, KeyCode::Esc) {
      return Ok(Some(EventResponse::Stop(Action::Back)));
    }

    Ok(None)
  }

  fn update(&mut self) {
    let Some(field) = self.fields.get(self.selected) else { return };
    if let Some(prompt) = self.prompt_states.get_mut(field) {
      prompt.1.write().unwrap().focus();
    }
  }

  pub fn up(&mut self) {
    if self.fields.is_empty() {
      return;
    }

    if self.inside {
      let Some(field) = self.fields.get(self.selected) else { return };
      let Some(page) = self.children.get_mut(field) else { return };
      page.up()
    } else {
      self.selected = self.selected.saturating_add(self.fields.len() - 1) % self.fields.len();
      self.update();
    }
  }

  pub fn down(&mut self) {
    if self.fields.is_empty() {
      return;
    }

    if self.inside {
      let Some(field) = self.fields.get(self.selected) else { return };
      let Some(page) = self.children.get_mut(field) else {
        self.selected = self.selected.saturating_add(1) % self.fields.len();
        self.update();
        return;
      };
      page.down()
    } else {
      self.selected = self.selected.saturating_add(1) % self.fields.len();
      self.update();
    }
  }

  pub fn submit(&mut self) {
    if self.fields.is_empty() {
      return;
    }

    if self.inside {
      let Some(field) = self.fields.get(self.selected) else {
        return;
      };

      if let Some(child) = self.children.get_mut(field) {
        child.submit()
      }
    } else {
      self.inside = true;
      self.update();
    }
  }

  pub fn to_json(&self) -> Result<serde_json::Value> {
    let mut map = serde_json::Map::new();

    for (k, v) in &self.prompt_states {
      let val = v.1.read().unwrap();
      let val = val.value();
      let val = match v.0 {
        SchemaEditorPromptType::String => serde_json::Value::String(val.to_string()),
        SchemaEditorPromptType::Password => serde_json::Value::String(val.to_string()),
        SchemaEditorPromptType::Int => {
          val.parse().map(|v| serde_json::Value::Number(v)).unwrap_or(serde_json::Value::Null)
        },
        SchemaEditorPromptType::Float => {
          val.parse().map(|v| serde_json::Value::Number(v)).unwrap_or(serde_json::Value::Null)
        },
        SchemaEditorPromptType::Bool => serde_json::Value::Bool(val == "t"),
      };

      match val {
        serde_json::Value::String(s) if s.is_empty() => {},
        serde_json::Value::Null => {},
        val => {
          map.insert(k.clone(), val);
        },
      }
    }

    for (k, v) in &self.children {
      let v = v.to_json()?;
      if !v.is_null() {
        map.insert(k.clone(), v);
      }
    }

    if map.is_empty() {
      return Ok(serde_json::Value::Null);
    }

    Ok(serde_json::Value::Object(map))
  }
}
impl<'a> SchemaEditorPageState<'a> {
  pub fn page(&self, path: &mut Vec<String>) -> Option<&SchemaEditorPageState<'a>> {
    let field = self.fields.get(self.selected)?;
    path.push(self.prop_name.clone());

    if self.inside {
      if let Some(child) = self.children.get(field) {
        child.page(path)
      } else {
        Some(self)
      }
    } else {
      Some(self)
    }
  }
}

impl SchemaEditorState<'_> {
  pub fn new(schema: Option<&Schema>, global_state: Arc<RwLock<GlobalState>>) -> Result<Self> {
    let root = schema.map(|schema| SchemaEditorPageState::new(String::from("root"), schema, global_state));
    let root = if let Some(root) = root { Some(root?) } else { None };
    Ok(Self { root, inside: false })
  }

  pub fn set_schema(&mut self, schema: Schema, global_state: Arc<RwLock<GlobalState>>) -> Result<()> {
    let root = SchemaEditorPageState::new(String::from("root"), &schema, global_state)?;
    self.root = Some(root);
    self.inside = false;
    Ok(())
  }

  pub fn clear(&mut self) {
    self.root = None;
    self.inside = false;
  }

  pub fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    if let Some(root) = self.root.as_mut() {
      let resp = root.handle_key_events(key);

      if matches!(resp, Ok(Some(EventResponse::Stop(Action::Back)))) {
        Ok(None)
      } else {
        resp
      }
    } else {
      Ok(None)
    }
  }

  pub fn up(&mut self) {
    if let Some(root) = self.root.as_mut() {
      root.up()
    }
  }

  pub fn down(&mut self) {
    if let Some(root) = self.root.as_mut() {
      root.down()
    }
  }

  pub fn submit(&mut self) {
    if let Some(root) = self.root.as_mut() {
      root.submit()
    }
  }

  pub fn to_json(&mut self) -> Result<serde_json::Value> {
    if let Some(root) = self.root.as_mut() {
      root.to_json()
    } else {
      Ok(serde_json::Value::Null)
    }
  }
}
impl<'a> SchemaEditorState<'a> {
  pub fn page(&self) -> Option<(Vec<String>, &SchemaEditorPageState<'a>)> {
    self
      .root
      .as_ref()
      .map(|p| {
        let mut path = Vec::new();
        p.page(&mut path).map(|p| (path, p))
      })
      .flatten()
  }
}
