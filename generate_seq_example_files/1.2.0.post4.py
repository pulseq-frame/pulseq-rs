import os

import numpy as np
np.int = int
np.float = float
np.complex = complex

import matplotlib.pyplot as plt
plt.show = plt.close

# Before version 1.3, pypulseq doesn't include its examples in the PyPI packet.
# The 5 example files are downloaded manually.

from pp_1_2_examples import write_epi_rs
from pp_1_2_examples import write_epi
from pp_1_2_examples import write_gre
from pp_1_2_examples import write_haste
from pp_1_2_examples import write_tse

os.replace("epi_rs_pypulseq.seq", "assets/1.2.0.post4/epi_rs_pypulseq.seq")
os.replace("epi_pypulseq.seq", "assets/1.2.0.post4/epi_pypulseq.seq")
os.replace("gre_pypulseq.seq", "assets/1.2.0.post4/gre_pypulseq.seq")
os.replace("haste_pypulseq.seq", "assets/1.2.0.post4/haste_pypulseq.seq")
os.replace("tse_pypulseq.seq", "assets/1.2.0.post4/tse_pypulseq.seq")
