use naga::back::glsl;

fn translate(source: &str, relaxed: bool) -> Result<String, glsl::Error> {
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
    let mut options = glsl::Options {
        version: glsl::Version::Embedded {
            version: 300,
            is_webgl: true,
        },
        ..Default::default()
    };
    options
        .writer_flags
        .set(glsl::WriterFlags::F16_AS_MEDIUMP, relaxed);
    let pipeline = glsl::PipelineOptions {
        shader_stage: naga::ShaderStage::Fragment,
        entry_point: "main".into(),
        multiview: None,
    };
    let mut out = String::new();
    glsl::Writer::new(
        &mut out,
        &module,
        &info,
        &options,
        &pipeline,
        Default::default(),
    )?
    .write()?;
    Ok(out)
}

const SHADER: &str = "enable f16; fn half_color(x: vec4<f16>) -> vec4<f16> { var c = x * 0.5h; return c; } @fragment fn main(@location(0) c: vec4<f32>) -> @location(0) vec4<f32> { return vec4<f32>(half_color(vec4<f16>(c))); }";

#[test]
fn mediump_declarations_and_unqualified_constructors() {
    let output = translate(SHADER, true).unwrap();
    assert!(output.contains("precision highp float;"));
    assert!(output.contains("mediump vec4 half_color"), "{output}");
    assert!(output.contains("mediump vec4 x"), "{output}");
    assert!(output.contains("mediump vec4 c"), "{output}");
    assert!(!output.contains("mediump vec4("), "{output}");
    assert!(!output.contains("mediump float("), "{output}");
}

#[test]
fn normal_glsl_rejects_half() {
    assert!(translate(SHADER, false).is_err());
}

#[test]
fn half_buffer_layout_is_rejected() {
    let source = "enable f16; struct U { color: vec4<f16> } @group(0) @binding(0) var<uniform> u: U; @fragment fn main() -> @location(0) vec4<f32> { return vec4<f32>(u.color); }";
    assert!(translate(source, true)
        .unwrap_err()
        .to_string()
        .contains("ABI"));
}

#[test]
fn half_bitcast_is_rejected() {
    let source = "enable f16; @fragment fn main(@location(0) x: vec2<f32>) -> @location(0) vec4<f32> { let h = f16(x.x); let bits = bitcast<u32>(f32(h)); return vec4<f32>(f32(bits)); }";
    assert!(translate(source, true)
        .unwrap_err()
        .to_string()
        .contains("bitcast"));
}
#[test]
fn inferred_half_scalar_requires_opt_in_and_gets_precision() {
    let source = "enable f16; @fragment fn main(@location(0) x: f32) -> @location(0) vec4<f32> { let h = f16(x); let y = h * 0.5h; return vec4<f32>(f32(y)); }";
    assert!(translate(source, false).is_err());
    let output = translate(source, true).unwrap();
    assert!(output.contains("mediump float"), "{output}");
}
