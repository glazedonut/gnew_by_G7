#!/bin/sh

. ./test-lib.sh

test_expect_success 'fast-forward merge' '
	test_commit init foo &&
	gnew checkout -b branch1 &&
	test_commit "add bar" bar &&
	gnew checkout main &&
	gnew merge branch1 &&
	gnew log >out &&
	grep "add bar" out
'
