# Buffer Sizing Guide

- **Const-sized messages:** stack `[0u8; MsgEncoder::compute_length()]`
- **Dynamic / ragged:** size with `*EncodedLength` / `compute_length_with_header(…)`,
  then encode into a claim/slot of that exact length — avoid oversize
  `vec![0u8; 4096]` “guess” buffers

See the feature-tour `demo_car_size_and_encode`.
