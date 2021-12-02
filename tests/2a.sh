#!/bin/sh

. ./test-lib.sh

test_expect_success 'checkout nonexistent branch fails' '
	! gnew checkout other 2>out &&
	grep "fatal: reference not found" out
'

test_expect_success 'checkout -b to an existing branch fails' '
	test_commit initial file1 &&
	! gnew checkout -b main 2>out &&
	grep "fatal: branch already exists" out
'
