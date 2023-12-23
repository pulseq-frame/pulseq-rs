import os

import numpy as np
np.int = int
np.float = float
np.complex = complex

import matplotlib.pyplot as plt
plt.show = plt.close

from pypulseq.seq_examples.scripts import write_epi
from pypulseq.seq_examples.scripts import write_epi_se
from pypulseq.seq_examples.scripts import write_epi_se_rs
from pypulseq.seq_examples.scripts import write_gre
from pypulseq.seq_examples.scripts import write_gre_label
from pypulseq.seq_examples.scripts import write_haste
from pypulseq.seq_examples.scripts import write_tse
from pypulseq.seq_examples.scripts import write_ute

print(os.getcwd())
os.replace("epi_pypulseq.seq", "assets/1.3.1.post1/epi.seq")
os.replace("epi_se_pypulseq.seq", "assets/1.3.1.post1/epi_se.seq")
os.replace("epi_se_rs_pypulseq.seq", "assets/1.3.1.post1/epi_se_rs.seq")
os.replace("gre_pypulseq.seq", "assets/1.3.1.post1/gre.seq")
os.replace("gre_label_pypulseq.seq", "assets/1.3.1.post1/gre_label.seq")
os.replace("haste_pypulseq.seq", "assets/1.3.1.post1/haste.seq")
os.replace("tse_pypulseq.seq", "assets/1.3.1.post1/tse.seq")
os.replace("ute_pypulseq.seq", "assets/1.3.1.post1/ute.seq")
