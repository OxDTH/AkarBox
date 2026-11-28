# Generates tiny.dng: a 64x48 RGGB Bayer mosaic wrapped as a DNG.
# Usage: pip install tifffile numpy && python make_tiny_dng.py tiny.dng
import sys, numpy as np, tifffile
# 64x48 Bayer RGGB mosaic with a horizontal gradient.
h, w = 48, 64
img = np.zeros((h, w), np.uint16)
x = np.linspace(500, 3500, w).astype(np.uint16)
img[0::2, 0::2] = x[0::2]          # R
img[0::2, 1::2] = 2000             # G
img[1::2, 0::2] = 2000             # G
img[1::2, 1::2] = x[::-1][1::2]    # B
extratags = [
    (50706, 'B', 4, (1, 4, 0, 0), True),        # DNGVersion
    (50708, 's', 0, 'AkarBox Test', True),      # UniqueCameraModel
    (33421, 'H', 2, (2, 2), True),              # CFARepeatPatternDim
    (33422, 'B', 4, (0, 1, 1, 2), True),        # CFAPattern RGGB
    (50717, 'I', 1, 4095, True),                # WhiteLevel
    (50721, '2i', 9, (10000,10000, 0,10000, 0,10000, 0,10000, 10000,10000, 0,10000, 0,10000, 0,10000, 10000,10000), True),  # ColorMatrix1 (identity)
    (50778, 'H', 1, 21, True),                  # CalibrationIlluminant1 = D65
]
tifffile.imwrite(sys.argv[1], img, photometric=32803, extratags=extratags, metadata=None)
