"""
**HSPICE result file input and conversion**
Rust implementation with PyO3 by Haiwen Zhang
"""

import _hspcie_tr0_parser

__all__ = [ 'hspice_tr0_read', 'hspice_tr0_to_raw' ]

def hspice_tr0_read(filename, debug=0):
	"""
	Returns a list with only one tuple as member (representing the results of 
	one analysis). 
	
	The tuple has the following members
	
	0. Simulation results tuple with following members
	  
	  If a variable was swept and the analysis repeated for every value in the 
	  sweep
	    
		0. The name of the swept parameter
		1. An array with the N values of the parameter
		2. A list with N dictionaries, one for every parameter value holding 
		   the simulation results where result name is the key and values are 
		   arrays. 
	  
	  If no variable was swept and the analysis was performed only once
	    
		0. ``None``
		1. ``None``
		2. A list with one dictionay as the only memebr. The dictionary holds 
		   the simulation results. The name of a result is the key while values 
		   are arrays. 
		   
	1. The name of the default scale array 
	2. ``None`` (would be the dictionary of non-default scale vector names)
	3. Title string
	4. Date string
	5. ``None`` (would be the plot name string)
	
	Returns ``None`` if an error occurs during reading. 
	"""
	return _hspcie_tr0_parser.tr0_read(filename, debug)

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
		>>> from tr0parser import hspice_tr0_to_raw
		>>> hspice_tr0_to_raw('simulation.tr0', 'simulation.raw')
		True
	"""
	return _hspcie_tr0_parser.tr0_to_raw(input_path, output_path, debug)