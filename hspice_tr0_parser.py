"""
**HSPICE result file input and conversion**
Rust implementation with PyO3 by Haiwen Zhang
"""

import hspicetr0parser as _hspicetr0parser

__all__ = ['hspice_tr0_read', 'hspice_tr0_to_raw']

def hspice_tr0_read(filename, data_type='numpy', debug=0):
	"""
	Read HSPICE binary file and return simulation data.
	
	Args:
		filename: Path to the HSPICE binary file (.tr0, .ac0, .sw0)
		data_type: Return data type, either 'numpy' (default) or 'list'
		           - 'numpy': Returns NumPy arrays (requires numpy)
		           - 'list': Returns Python native lists (no numpy dependency)
		debug: Debug level (0=quiet, 1=info, 2=verbose)
	
	Returns:
		A list with one tuple containing:
		
		0. Simulation results tuple:
		   - If swept: (sweep_name, sweep_values, [data_dicts])
		   - If not swept: (None, None, [data_dict])
		1. Scale name (e.g., "TIME")
		2. None (placeholder)
		3. Title string
		4. Date string
		5. None (placeholder)
		
		Returns None if an error occurs.
	
	Example:
		>>> from hspice_tr0_parser import hspice_tr0_read
		>>> # NumPy arrays (default)
		>>> result = hspice_tr0_read('simulation.tr0')
		>>> data = result[0][0][2][0]
		>>> time = data['TIME']  # numpy.ndarray
		>>> 
		>>> # Python lists (no numpy dependency)
		>>> result = hspice_tr0_read('simulation.tr0', data_type='list')
		>>> time = result[0][0][2][0]['TIME']  # list
	"""
	if data_type == 'numpy':
		return _hspicetr0parser.tr0_read_numpy(filename, debug)
	elif data_type == 'list':
		return _hspicetr0parser.tr0_read_native(filename, debug)
	else:
		raise ValueError(f"data_type must be 'numpy' or 'list', got '{data_type}'")

def hspice_tr0_to_raw(input_path, output_path, debug=0):
	"""
	Convert HSPICE binary .tr0 file to SPICE3 binary raw format.
	
	Args:
		input_path: Path to the input HSPICE .tr0 file
		output_path: Path for the output SPICE3 .raw file
		debug: Debug level (0=quiet, 1=info, 2=verbose)
	
	Returns:
		True if conversion succeeded, False otherwise.
	
	Example:
		>>> from hspice_tr0_parser import hspice_tr0_to_raw
		>>> hspice_tr0_to_raw('simulation.tr0', 'simulation.raw')
		True
	"""
	return _hspicetr0parser.tr0_to_raw(input_path, output_path, debug)