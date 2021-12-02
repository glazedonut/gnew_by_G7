#!/bin/sh

. ./test-lib.sh

test_expect_success 'checkout updates working tree' '
	test_commit foo foo.txt &&
	gnew checkout -b other &&
	test_commit foobar foo.txt &&
	gnew checkout main &&
	grep foo foo.txt &&
	gnew checkout other &&
	grep foobar foo.txt
'
