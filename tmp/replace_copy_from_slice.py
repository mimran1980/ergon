#!/usr/bin/env python3

PATH = "/Users/imran/RustroverProjects/ErgoSBE/sbe/src/codegen.rs"

with open(PATH, 'r') as f:
    content = f.read()

replacements = []

# 1. Message header template (lines 3275, 3294) - {4}=header_size
replacements.append((
    '                     buf[pos..pos + {}].copy_from_slice(&Self::HEADER_TEMPLATE);\\n",',
    '                     #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                     {{ buf[pos..pos + {4}].copy_from_slice(&Self::HEADER_TEMPLATE); }}\\n\\\n'
    '                     #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                     {{ unsafe {{ core::ptr::copy_nonoverlapping(Self::HEADER_TEMPLATE.as_ptr(), buf.as_mut_ptr().add(pos), {4}); }} }}\\n",'
))

# 2. Message array setter (line 3332) - {7},{8},{9}=prim_size
replacements.append((
    '                                     self.buf[offset + idx * {}..offset + idx * {} + {}].copy_from_slice(&val_bytes);\\n\\',
    '                                     #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                                     {{ self.buf[offset + idx * {7}..offset + idx * {8} + {9}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '                                     #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                                     {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset).add(idx * {7}), {9}); }} }}\\n\\'
))

# 3. Message composite (line 3368) - {4}=comp_size, unique: val.0
replacements.append((
    '                             self.buf[offset..offset + {}].copy_from_slice(&val.0);\\n\\',
    '                             #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                             {{ self.buf[offset..offset + {4}].copy_from_slice(&val.0); }}\\n\\\n'
    '                             #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                             {{ unsafe {{ core::ptr::copy_nonoverlapping(val.0.as_ptr(), self.buf.as_mut_ptr().add(offset), {4}); }} }}\\n\\'
))

# 4. Message scalar (line 3353) - 3-line: message_start + 13-sp indent + val.to_
# {2}=header_size, {3}=offset, {4}=order_suffix, {5}=prim_size
replacements.append((
    '             let offset = self.message_start + {} + {};\\n\\\n'
    '             let val_bytes = val.to_{}_bytes();\\n\\\n'
    '             self.buf[offset..offset + {}].copy_from_slice(&val_bytes);\\n\\',
    '             let offset = self.message_start + {2} + {3};\\n\\\n'
    '             let val_bytes = val.to_{4}_bytes();\\n\\\n'
    '             #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '             {{ self.buf[offset..offset + {5}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '             #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '             {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset), {5}); }} }}\\n\\'
))

# 5. Message enum (line 3387) - 3-line: message_start + 9-sp indent + val.0.to_
# {2}=header_size, {3}=offset, {4}=order_suffix, {5}=prim_size
replacements.append((
    '         let offset = self.message_start + {} + {};\\n\\\n'
    '         let val_bytes = val.0.to_{}_bytes();\\n\\\n'
    '         self.buf[offset..offset + {}].copy_from_slice(&val_bytes);\\n\\',
    '         let offset = self.message_start + {2} + {3};\\n\\\n'
    '         let val_bytes = val.0.to_{4}_bytes();\\n\\\n'
    '         #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '         {{ self.buf[offset..offset + {5}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '         #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '         {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset), {5}); }} }}\\n\\'
))

# 6. Message set (line 3404) - 3-line: message_start + 13-sp indent + val.0.to_
# {2}=header_size, {3}=offset, {4}=order_suffix, {5}=prim_size
replacements.append((
    '             let offset = self.message_start + {} + {};\\n\\\n'
    '             let val_bytes = val.0.to_{}_bytes();\\n\\\n'
    '             self.buf[offset..offset + {}].copy_from_slice(&val_bytes);\\n\\',
    '             let offset = self.message_start + {2} + {3};\\n\\\n'
    '             let val_bytes = val.0.to_{4}_bytes();\\n\\\n'
    '             #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '             {{ self.buf[offset..offset + {5}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '             #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '             {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset), {5}); }} }}\\n\\'
))

# 7. Message group dim + count (lines 3458-3459) - match through Ok({}Encoder {{
# Dim: {9}=dim_size, {10}=g_pascal. Count: {11}={12}=num_offset, {13}=num_size, {14}=order_suffix
# Wrap: {15}=g_pascal, {16}=dim_size, return: {17}=name
replacements.append((
    '                         self.buf[self.pos..self.pos + {}].copy_from_slice(&{}Encoder::GROUP_DIM_TEMPLATE);\\n\\\n'
    '                         self.buf[self.pos + {}..self.pos + {} + {}].copy_from_slice(&count.to_{}_bytes());\\n\\\n'
    '                         let mut group = {}Encoder::wrap(self.buf, self.pos + {}, count);\\n\\\n'
    '                         f(&mut group);\\n\\\n'
    '                         Ok({}Encoder {{',
    '                         #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                         {{ self.buf[self.pos..self.pos + {9}].copy_from_slice(&{10}Encoder::GROUP_DIM_TEMPLATE); }}\\n\\\n'
    '                         #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                         {{ unsafe {{ core::ptr::copy_nonoverlapping({10}Encoder::GROUP_DIM_TEMPLATE.as_ptr(), self.buf.as_mut_ptr().add(self.pos), {9}); }} }}\\n\\\n'
    '                         #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                         {{ self.buf[self.pos + {11}..self.pos + {12} + {13}].copy_from_slice(&count.to_{14}_bytes()); }}\\n\\\n'
    '                         #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                         {{ unsafe {{ core::ptr::copy_nonoverlapping(count.to_{14}_bytes().as_ptr(), self.buf.as_mut_ptr().add(self.pos).add({11}), {13}); }} }}\\n\\\n'
    '                         let mut group = {15}Encoder::wrap(self.buf, self.pos + {16}, count);\\n\\\n'
    '                         f(&mut group);\\n\\\n'
    '                         Ok({17}Encoder {{'
))

# 8. Message var-data prefix + body (lines 3511, 3513 + 3530, 3532) - 17-sp indent
# {3}=prefix_size (len_bytes copy), {4}=prefix_size (start)
replacements.append((
    '                 self.buf[self.pos..self.pos + {}].copy_from_slice(&len_bytes);\\n\\\n'
    '                 let start = self.pos + {};\\n\\\n'
    '                 self.buf[start..start + data.len()].copy_from_slice(data);\\n\\',
    '                 #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                 {{ self.buf[self.pos..self.pos + {3}].copy_from_slice(&len_bytes); }}\\n\\\n'
    '                 #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                 {{ unsafe {{ core::ptr::copy_nonoverlapping(len_bytes.as_ptr(), self.buf.as_mut_ptr().add(self.pos), {3}); }} }}\\n\\\n'
    '                 let start = self.pos + {4};\\n\\\n'
    '                 #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                 {{ self.buf[start..start + data.len()].copy_from_slice(data); }}\\n\\\n'
    '                 #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                 {{ unsafe {{ core::ptr::copy_nonoverlapping(data.as_ptr(), self.buf.as_mut_ptr().add(start), data.len()); }} }}\\n\\'
))

# 9. Entry array setter (line 3731) - {6},{7},{8}=prim_size
replacements.append((
    '                                     self.buf[offset + idx * {}..offset + idx * {} + {}].copy_from_slice(&val_bytes);\\n\\',
    '                                     #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                                     {{ self.buf[offset + idx * {6}..offset + idx * {7} + {8}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '                                     #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                                     {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset).add(idx * {6}), {8}); }} }}\\n\\'
))

# 10. Entry composite (line 3758) - {3}=comp_size, unique: val.0 with entry_start
replacements.append((
    '                         let offset = self.entry_start + {};\\n\\\n'
    '                         self.buf[offset..offset + {}].copy_from_slice(&val.0);\\n\\',
    '                         let offset = self.entry_start + {2};\\n\\\n'
    '                         #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                         {{ self.buf[offset..offset + {3}].copy_from_slice(&val.0); }}\\n\\\n'
    '                         #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                         {{ unsafe {{ core::ptr::copy_nonoverlapping(val.0.as_ptr(), self.buf.as_mut_ptr().add(offset), {3}); }} }}\\n\\'
))

# 11. Entry scalar (line 3743) - 3-line: entry_start + 29-sp + val.to_
# {2}=offset, {3}=order_suffix, {4}=prim_size
replacements.append((
    '                             let offset = self.entry_start + {};\\n\\\n'
    '                             let val_bytes = val.to_{}_bytes();\\n\\\n'
    '                             self.buf[offset..offset + {}].copy_from_slice(&val_bytes);\\n\\',
    '                             let offset = self.entry_start + {2};\\n\\\n'
    '                             let val_bytes = val.to_{3}_bytes();\\n\\\n'
    '                             #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                             {{ self.buf[offset..offset + {4}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '                             #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                             {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset), {4}); }} }}\\n\\'
))

# 12. Entry enum (line 3774) - 3-line: entry_start + 9-sp + val.0.to_
# {2}=offset, {3}=order_suffix, {4}=prim_size
replacements.append((
    '         let offset = self.entry_start + {};\\n\\\n'
    '         let val_bytes = val.0.to_{}_bytes();\\n\\\n'
    '         self.buf[offset..offset + {}].copy_from_slice(&val_bytes);\\n\\',
    '         let offset = self.entry_start + {2};\\n\\\n'
    '         let val_bytes = val.0.to_{3}_bytes();\\n\\\n'
    '         #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '         {{ self.buf[offset..offset + {4}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '         #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '         {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset), {4}); }} }}\\n\\'
))

# 13. Entry set (line 3790) - 3-line: entry_start + 13-sp + val.0.to_
# {2}=offset, {3}=order_suffix, {4}=prim_size
replacements.append((
    '             let offset = self.entry_start + {};\\n\\\n'
    '             let val_bytes = val.0.to_{}_bytes();\\n\\\n'
    '             self.buf[offset..offset + {}].copy_from_slice(&val_bytes);\\n\\',
    '             let offset = self.entry_start + {2};\\n\\\n'
    '             let val_bytes = val.0.to_{3}_bytes();\\n\\\n'
    '             #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '             {{ self.buf[offset..offset + {4}].copy_from_slice(&val_bytes); }}\\n\\\n'
    '             #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '             {{ unsafe {{ core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), self.buf.as_mut_ptr().add(offset), {4}); }} }}\\n\\'
))

# 14. Entry group dim + count (lines 3815-3816) - match through Ok(self)
# Dim: {4}=dim_size, {5}=ng_pascal
# Count: {6}={7}=num_offset, {8}=num_size, {9}=order_suffix
# Wrap: {10}=ng_pascal, {11}=dim_size
replacements.append((
    '                         self.buf[self.pos..self.pos + {}].copy_from_slice(&{}Encoder::GROUP_DIM_TEMPLATE);\\n\\\n'
    '                         self.buf[self.pos + {}..self.pos + {} + {}].copy_from_slice(&count.to_{}_bytes());\\n\\\n'
    '                         let mut group = {}Encoder::wrap(self.buf, self.pos + {}, count);\\n\\\n'
    '                         f(&mut group);\\n\\\n'
    '                         self.pos = group.pos;\\n\\\n'
    '                         Ok(self)',
    '                         #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                         {{ self.buf[self.pos..self.pos + {4}].copy_from_slice(&{5}Encoder::GROUP_DIM_TEMPLATE); }}\\n\\\n'
    '                         #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                         {{ unsafe {{ core::ptr::copy_nonoverlapping({5}Encoder::GROUP_DIM_TEMPLATE.as_ptr(), self.buf.as_mut_ptr().add(self.pos), {4}); }} }}\\n\\\n'
    '                         #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                         {{ self.buf[self.pos + {6}..self.pos + {7} + {8}].copy_from_slice(&count.to_{9}_bytes()); }}\\n\\\n'
    '                         #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                         {{ unsafe {{ core::ptr::copy_nonoverlapping(count.to_{9}_bytes().as_ptr(), self.buf.as_mut_ptr().add(self.pos).add({6}), {8}); }} }}\\n\\\n'
    '                         let mut group = {10}Encoder::wrap(self.buf, self.pos + {11}, count);\\n\\\n'
    '                         f(&mut group);\\n\\\n'
    '                         self.pos = group.pos;\\n\\\n'
    '                         Ok(self)'
))

# 15. Entry var-data prefix + body (lines 3841, 3843) - 21-sp indent
# {4}=prefix_size (len_bytes copy), {5}=prefix_size (start)
replacements.append((
    '                     self.buf[self.pos..self.pos + {}].copy_from_slice(&len_bytes);\\n\\\n'
    '                     let start = self.pos + {};\\n\\\n'
    '                     self.buf[start..start + data.len()].copy_from_slice(data);\\n\\',
    '                     #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                     {{ self.buf[self.pos..self.pos + {4}].copy_from_slice(&len_bytes); }}\\n\\\n'
    '                     #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                     {{ unsafe {{ core::ptr::copy_nonoverlapping(len_bytes.as_ptr(), self.buf.as_mut_ptr().add(self.pos), {4}); }} }}\\n\\\n'
    '                     let start = self.pos + {5};\\n\\\n'
    '                     #[cfg(not(feature = \\"disable-bounds-checks\\"))]\\n\\\n'
    '                     {{ self.buf[start..start + data.len()].copy_from_slice(data); }}\\n\\\n'
    '                     #[cfg(feature = \\"disable-bounds-checks\\")]\\n\\\n'
    '                     {{ unsafe {{ core::ptr::copy_nonoverlapping(data.as_ptr(), self.buf.as_mut_ptr().add(start), data.len()); }} }}\\n\\'
))

# Apply all replacements
for i, (old, new) in enumerate(replacements, 1):
    count = content.count(old)
    if count == 0:
        print(f"WARNING: Replacement {i} matched 0 times -- SKIPPING")
        print(f"  old begin: {old[:80]}")
        continue
    content = content.replace(old, new)
    print(f"Replacement {i}: matched {count} time(s)")

with open(PATH, 'w') as f:
    f.write(content)

print(f"\nDone! Wrote {PATH}")
