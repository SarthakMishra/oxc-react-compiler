use napi_derive::napi;

#[napi]
pub fn transform(source: String, filename: String) -> napi::Result<String> {
    // TODO: Parse with OXC, run compiler pipeline, return transformed source.
    let _ = (&source, &filename);
    Ok(source)
}
