# PyPulseq_rfshim

pulseq-rs supports the pTx extension by Martin Freudensprung:
https://gitlab.cs.fau.de/mrzero/pypulseq_rfshim

This extension differentiates itself bvy the file format (either 1.3.90 or 1.4.5).
Since the changes are orthogonal to the rest of the pulseq format, pulseq-rs supports them for all versions.


## rf_shim changes

The only change introduced by PyPulseq_rfshim is the introduction of an shim_mag_ID and shim_phase_ID in the pulse definition.
These refer to shape_IDs. The shapes are in no way different from those used by gradients, pulses or time samples.

Pulses returned by pulseq-rs have an optional shim array that is filled if the IDs are encountered in the .seq file.
Both IDs must be defined or none, it is an parsing error if only one is found.
The amount of elements in the elements is not checked in any way.
