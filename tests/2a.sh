#!/bin/sh

. ./test-lib.sh

test_expect_success 'add adds file to tracklist' '
	echo foo > foo.txt &&
	gnew add foo.txt &&
	grep foo .gnew/tracklist
'

test_expect_success 'add recursively adds directory contents to tracklist' '
	mkdir bar &&
	echo bar > bar/bar.txt &&
	mkdir bar/baz &&
	echo baz > bar/baz/baz.txt &&
	gnew add bar &&
	grep bar/bar.txt .gnew/tracklist &&
	grep bar/baz/baz.txt .gnew/tracklist
'
