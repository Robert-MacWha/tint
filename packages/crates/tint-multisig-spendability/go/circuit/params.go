package circuit

// NSigners and Threshold are compile-time constants (the circuit's shape is
// fixed, matching how join_split.rs hardcodes N_INPUTS/N_OUTPUTS as plain
// consts rather than runtime parameters). Demo defaults, trivially
// adjustable: a 2-of-3 multisig.
const (
	NSigners  = 3
	Threshold = 2
)
