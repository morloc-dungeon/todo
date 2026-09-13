# Build/test conventions for a morloc-dungeon example.
#
# dungeon-master drives exactly these three targets:
#
#   make build   build the program; nonzero exit on any build failure
#   make test    run it and diff against test/exp.txt; nonzero exit on mismatch
#   make clean   remove build products
#
# `test` depends on `build`, so `make test` builds and checks in one step.
# The `tui` and `app` commands need a terminal and a port, so `test` covers
# the command line only.

EXE := todo
SRC := main.loc

.PHONY: build test clean

build:
	morloc make -o $(EXE) $(SRC)

test: build
	: > test/obs.txt
	rm -f test/todo.json
	./$(EXE) add 'write the docs' -p 3 -t docs -t urgent -F test/todo.json >> test/obs.txt
	./$(EXE) add 'buy milk'                              -F test/todo.json >> test/obs.txt
	./$(EXE) add 'file taxes' -p 2 -x                    -F test/todo.json >> test/obs.txt
	./$(EXE) list                                        -F test/todo.json >> test/obs.txt
	./$(EXE) list -k                                     -F test/todo.json >> test/obs.txt
	./$(EXE) list -n                                     -F test/todo.json >> test/obs.txt
	./$(EXE) list -u                                     -F test/todo.json >> test/obs.txt
	./$(EXE) list -m 2                                   -F test/todo.json >> test/obs.txt
	./$(EXE) check 1                                     -F test/todo.json >> test/obs.txt
	./$(EXE) sort                                        -F test/todo.json >> test/obs.txt
	./$(EXE) clean                                       -F test/todo.json >> test/obs.txt
	./$(EXE) drop 0                                      -F test/todo.json >> test/obs.txt
	./$(EXE) -f json list                                -F test/todo.json >> test/obs.txt
	diff -u test/exp.txt test/obs.txt

clean:
	rm -rf $(EXE) $(EXE)-build
	rm -f test/obs.txt test/todo.json
