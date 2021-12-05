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

test_expect_success 'commit correctly creates objects' '
	gnew commit "init" &&	
	ls .gnew/objects | wc -l | grep 7
'

test_expect_success 'commit correctly sets the branch head' '
	test_commit initial file >gout &&
	cat .gnew/heads/main > cout &&
	diff gout cout
'

test_expect_success 'checkout updates working tree' '
	mkdir ch &&
	cd ch &&
	gnew init &&
	test_commit one one.txt &&
	gnew checkout -b other &&
	test_commit two one.txt &&
	gnew checkout main &&
	grep one one.txt &&
	gnew checkout other &&
	grep two one.txt
'
