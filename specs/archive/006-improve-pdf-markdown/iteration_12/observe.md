# OODA-12: Observe - Undocumented Constants in block_classifier.rs

## Current State

Several constants in `block_classifier.rs` lack WHY comments:

1. **line 133**: `ratio >= 2.0` - heading level 1 threshold
   - No explanation why 2.0x body size

2. **line 135**: `ratio >= 1.7` - heading level 2 threshold
   - No explanation why 1.7x body size

3. **line 291**: `>= 0.5` - uppercase ratio for all-caps section check
   - No explanation why 50%

## Evidence

Many constants already have good WHY comments:

- ✅ line 19: Conservative header ratio explained
- ✅ line 127: 1.50x threshold explained
- ✅ line 164: Bullet character variety explained
- ❌ line 133: 2.0 undocumented
- ❌ line 135: 1.7 undocumented
- ❌ line 291: 0.5 undocumented

## Analysis

These constants control:

- Heading level classification (H1 vs H2)
- All-caps section detection (e.g., "ABSTRACT", "REFERENCES")
