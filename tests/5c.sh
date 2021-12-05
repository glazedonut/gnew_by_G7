#!/bin/sh

. ./test-lib.sh

test_expect_success 'three-way merge' '
	test_commit init foo &&
	gnew checkout -b branch1 &&
	printf "change on branch1\ninit\n" >foo &&
	gnew commit foo &&
	gnew checkout main &&
	printf "init\nchange on main\n" >foo &&
	gnew commit foo &&
	gnew merge branch1 &&
	gnew commit "merge branch1" &&
	printf "change on branch1\ninit\nchange on main\n" >expect &&
	gnew cat HEAD foo >out &&
	diff expect out
'
