#!/bin/sh

. ./test-lib.sh

test_expect_success 'pull correctly pulls a remote branch' '
	mkdir remote &&
	cd remote &&
	gnew init > /dev/null &&
	test_commit init file.txt &&
	cp -R . ../local &&
	test_commit foo foo.txt &&
	cd ../local &&
	gnew pull ../remote &&
	gnew log | grep foo	
'

test_expect_success 'push correctly pushes to a remote branch' '
	test_commit bar bar.txt &&
	gnew push ../remote &&
	cd ../remote &&
	gnew log | grep bar
'

test_expect_success 'pull -a correctly pulls from all remote branches' '
	gnew checkout -b test &&
	test_commit baz baz.txt &&
	cd ../local &&
	gnew pull -a ../remote &&
	gnew checkout test &&
	gnew log | grep baz
'

test_expect_success 'push -a correctly pushes to all remote branches' '
	gnew checkout -b another &&
	test_commit whatever test.txt &&
	gnew push -a ../remote &&
	cd ../remote &&
	gnew checkout another &&
	gnew log | grep whatever
'
