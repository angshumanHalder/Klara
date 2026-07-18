struct VertIn {
  @location(0) pos: vec2<f32>,
  @location(1) color: vec4<f32>,
}

struct VertOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertIn) -> VertOut {
    return VertOut(vec4<f32>(in.pos, 0.0, 1.0), in.color);
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
  return in.color;
}
