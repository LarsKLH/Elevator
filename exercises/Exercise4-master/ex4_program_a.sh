#! /usr/bin/bash

sleep_time=0.5

polls_before_dead=4

counting_file_location=/home/gia2/yuuuuge_programs/yuge_stud/sanntidsprogramering/git_repo_dir/TTK4515-2025-Gruppe26/exercises/Exercise4-master/counting_file.txt

prev_number="0"

changeless_checks=0

echo $prev_number

while true;
do
	latest_number=$(tail -n 1 $counting_file_location) 
	
	if [ "$prev_number" != "$latest_number" ]; then
		echo "Last number different then prev: $prev_number, new latest: $latest_number"
		prev_number=$latest_number
		changeless_checks=0
	else
		echo "Last number not different then prev: $prev_number"
		((changeless_checks++))
	fi

	if [ $changeless_checks -gt $polls_before_dead  ]; then
		echo "The variable has been unchanged for $changeless_checks checks"
	fi

	sleep $sleep_time
done

