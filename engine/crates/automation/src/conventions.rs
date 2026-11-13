//! Shared command/inspection/preview entry points, using the existing confined tool path.
use serde_json::{Value, json};

use crate::{Args, Automation, Content, Result, ToolError, encode_png, failed, info, tools};
use pdfcraft_engine::commands;
use pdfcraft_render::{RenderRequest, RequestKind};

/// The most steps one `command_batch` runs.
pub(crate) const MAX_BATCH_STEPS: usize = 1000;

/// Why `id` has no tool to run: a tool name passed as a command id, an id the registry doesn't
/// know, or a real command that nothing automates yet. They need different fixes from the caller.
fn no_tool_for(id: &str) -> ToolError {
    if commands::command(id).is_some() {
        return failed(format!("perintah {id:?} tidak memiliki tool headless; lihat command_list"));
    }
    match tools::find(id) {
        Some(def) => match def.command {
            Some(command) => failed(format!("{id:?} adalah tool, bukan id perintah; gunakan id perintah {command:?}, atau panggil tool secara langsung")),
            None => failed(format!("{id:?} adalah tool, bukan id perintah; panggil secara langsung (id perintah ada di command_list)")),
        },
        None => failed(format!("id perintah tidak dikenal {id:?}; lihat command_list")),
    }
}

impl Automation {
    pub(crate) fn command_run(&mut self, args: &Args) -> Result<Vec<Content>> {
        let id = args.str("id")?;
        let tool = tools::tool_for_command(id).ok_or_else(|| no_tool_for(id))?;
        let def = tools::find(tool).ok_or_else(|| failed("tool perintah tidak tersedia"))?;
        let mut params = match args.get("params") {
            None | Some(Value::Null) => serde_json::Map::new(),
            Some(Value::Object(params)) => params.clone(),
            Some(_) => return Err(ToolError::InvalidArgs("params harus berupa objek".into())),
        };
        let props = def.input_schema.get("properties").and_then(Value::as_object).ok_or_else(|| failed("tool tidak memiliki skema parameter"))?;
        let accepted = props.keys().map(String::as_str).collect::<Vec<_>>().join(", ");
        let mut warnings = Vec::new();
        params.retain(|key, _| {
            if props.contains_key(key) {
                return true;
            }
            warnings.push(format!("param tidak dikenal {key:?} untuk {id} (yang diterima: {accepted})"));
            false
        });
        // This preserves the task tool's validation, atomic writes and --root confinement.
        let mut content = self.call(tool, &Value::Object(params))?;
        if !warnings.is_empty() {
            if let Some(object) = content.iter_mut().find_map(|c| match c {
                Content::Json(v) => v.as_object_mut(),
                _ => None,
            }) {
                let mut all = match object.remove("warnings") {
                    Some(Value::Array(old)) => old,
                    Some(old) => vec![old],
                    None => Vec::new(),
                };
                all.extend(warnings.into_iter().map(Value::String));
                object.insert("warnings".into(), Value::Array(all));
            } else {
                content.push(Content::Json(json!({"warnings":warnings})));
            }
        }
        Ok(content)
    }

    pub(crate) fn command_batch(&mut self, args: &Args) -> Result<Value> {
        let steps = args.get("steps").and_then(Value::as_array).ok_or_else(|| ToolError::InvalidArgs("steps harus berupa array".into()))?;
        if steps.len() > MAX_BATCH_STEPS {
            return Err(ToolError::InvalidArgs(format!("satu batch maksimal {MAX_BATCH_STEPS} langkah ({} diberikan); bagi menjadi beberapa batch", steps.len())));
        }
        // Check every envelope before applying any edit.
        for step in steps {
            let a = Args(step);
            a.str("id")?;
            if a.get("params").is_some_and(|p| !p.is_object() && !p.is_null()) {
                return Err(ToolError::InvalidArgs("params langkah harus berupa objek".into()));
            }
        }
        let stop = args.opt_bool("stop_on_error")?.unwrap_or(true);
        let (mut completed, mut failed, mut results) = (0usize, 0usize, Vec::new());
        for step in steps {
            match self.command_run(&Args(step)) {
                Ok(content) => {
                    completed += 1;
                    let result = content.into_iter().find_map(|c| match c {
                        Content::Json(v) => Some(v),
                        _ => None,
                    });
                    results.push(json!({"ok":true,"result":result}));
                }
                Err(e) => {
                    failed += 1;
                    results.push(json!({"ok":false,"error":e.to_string()}));
                    if stop {
                        break;
                    }
                }
            }
        }
        Ok(json!({"completed":completed,"failed":failed,"results":results}))
    }

    pub(crate) fn doc_inspect(&self, args: &Args) -> Result<Value> {
        if args.opt_int("doc")?.is_some() {
            return Ok(info(self.doc(args)?));
        }
        Ok(json!({"documents":self.session.docs().iter().map(info).collect::<Vec<_>>()}))
    }

    pub(crate) fn render_preview(&mut self, args: &Args) -> Result<Content> {
        let id = if args.opt_int("doc")?.is_some() {
            self.doc(args)?.id
        } else {
            match self.session.docs() {
                [doc] => doc.id,
                _ => return Err(failed("isi doc dari doc_list (satu dokumen terbuka dipilih otomatis)")),
            }
        };
        let page = args.opt_int("page")?.unwrap_or(1);
        let max = args.opt_int("max_side")?.unwrap_or(1024);
        if !(1..=4096).contains(&max) {
            return Err(ToolError::InvalidArgs("max_side harus antara 1 dan 4096".into()));
        }
        let page = usize::try_from(page).ok().and_then(|p| p.checked_sub(1)).ok_or_else(|| failed("page harus minimal 1"))?;
        let doc = self.session.get(id).ok_or_else(|| failed("dokumen tidak ada"))?;
        let size = doc.info.pages.get(page).ok_or_else(|| failed("halaman di luar jangkauan"))?;
        let side = size.width.max(size.height);
        if !side.is_finite() || side <= 0.0 {
            return Err(failed("halaman memiliki dimensi tidak valid"));
        }
        let scale = (max as f32 / side).min(1.0);
        let out = self.renderer(id)?.render(RenderRequest { page, kind: RequestKind::Pixels, tile: None, scale, tag: 0, preview: false });
        if let Some(error) = out.error {
            return Err(failed(error));
        }
        let data = encode_png(out.width, out.height, &out.rgba)?;
        Ok(Content::Png { data, width: out.width, height: out.height })
    }
}
