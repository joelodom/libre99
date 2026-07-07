# TI-99/4A book — build & verification (thin delegate; the scripts are the
# canonical path because they need only sh + cargo, which both workstations
# have — `make` is not installed everywhere this book is written).
#
# `make setup`  : build the project toolchain (libre99asm, emulator, bench99)
# `make verify` : assemble every .a99 under code/ with libre99asm; build bench99
# `make clean`  : remove build artifacts

.PHONY: setup verify clean
setup:
	sh setup.sh

verify:
	sh verify.sh

clean:
	rm -rf build code/bench/target
