import os

import numpy as np
np.int = int
np.float = float
np.complex = complex

import matplotlib.pyplot as plt
plt.show = plt.close

# Doesn't run because some block is not on the raster time
# from pypulseq.seq_examples.scripts import write_2Dt1_mprage
# from pypulseq.seq_examples.scripts import write_3Dt1_mprage
from pypulseq.seq_examples.scripts import write_MPRAGE
from pypulseq.seq_examples.scripts import write_epi
from pypulseq.seq_examples.scripts import write_epi_label
from pypulseq.seq_examples.scripts import write_epi_se
from pypulseq.seq_examples.scripts import write_epi_se_rs
from pypulseq.seq_examples.scripts import write_gre
from pypulseq.seq_examples.scripts import write_gre_label
from pypulseq.seq_examples.scripts import write_haste
from pypulseq.seq_examples.scripts import write_radial_gre
from pypulseq.seq_examples.scripts import write_tse
from pypulseq.seq_examples.scripts import write_ute

# os.replace("2d_mprage_pypulseq.seq", "assets/1.4.0/2d_t1_mprage.seq")
# os.replace("256_3d_t1_mprage_pypulseq.seq", "assets/1.4.0/3d_t1_mprage.seq")
write_MPRAGE.main(False, True, "assets/1.4.0/mprage.seq")
write_epi.main(False, True, "assets/1.4.0/epi.seq")
write_epi_label.main(False, True, "assets/1.4.0/epi_label.seq")
write_epi_se.main(False, True, "assets/1.4.0/epi_se.seq")
write_epi_se_rs.main(False, True, "assets/1.4.0/epi_se_rs.seq")
write_gre.main(False, True, "assets/1.4.0/gre.seq")
write_gre_label.main(False, True, "assets/1.4.0/gre_label.seq")
write_haste.main(False, True, "assets/1.4.0/haste.seq")
write_radial_gre.main(False, True, "assets/1.4.0/gre_radial.seq")
write_tse.main(False, True, "assets/1.4.0/tse.seq")
write_ute.main(False, True, "assets/1.4.0/ute.seq")
