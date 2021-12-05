#!/bin/sh

. ./test-lib.sh

test_expect_success 'merge fails if nothing to merge' '
	test_commit init foo &&
	gnew checkout -b branch1 &&
	! gnew merge main 2>out &&
	grep "fatal: nothing to merge" out
'
