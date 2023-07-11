import difflib
import subprocess
import json

def run_command(command):
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, shell=True)
    output, error = process.communicate()
    return output.decode().strip()

def remove_first_line(string):
    lines = string.split('\n', 1)  # Split string into lines, maximum split = 1
    if len(lines) > 1:
        return '\n'.join(lines[1:])  # Join lines starting from the second line
    else:
        return ''  # If there is only one line or the string is empty, return an empty string

def remove_last_line(string):
    lines = string.split('\n')
    if len(lines) > 1:
        return '\n'.join(lines[:-1])  # Join lines excluding the last line
    else:
        return ''  # If there is only one line or the string is empty, return an empty string


# Define the commands to run
command1 = "/home/ubuntu/kani/scripts/kani --enable-unstable --no-unwinding-checks --output-format=old /home/ubuntu/kani/tests/coverage/reach/assert_eq/unreachable/test.rs --cbmc-args --cover location --json-ui"
command2 = "/home/ubuntu/kani/scripts/kani --enable-unstable --output-format=old /home/ubuntu/kani/tests/coverage/reach/assert_eq/unreachable/test.rs --cbmc-args --json-ui"

# Run the commands and get their outputs
output1 = run_command(command1)
output2 = run_command(command2)

output1_cleaned = remove_last_line(remove_first_line(output1))
output2_cleaned = remove_last_line(remove_first_line(output2))

# # outputs in json
json_output1 = json.loads(output1_cleaned)
json_output2 = json.loads(output2_cleaned)

# find reachability checks in output2
found_reachability_check = []
for item in json_output2:
   if 'result' in str(item):
    for prop in item['result']:
        if 'reachability_check' in str(prop):
            found_reachability_check.append(prop)

unreachable_line_numbers = []
for check in found_reachability_check:
    unreachable_line_numbers.append(check['sourceLocation']['line'])

# print(unreachable_line_numbers)

# find reachability checks in output1
found_reachability_check_output1 = []
for item in json_output1:
   if 'goals' in str(item):
    for prop in item['goals']:
        if prop['sourceLocation']['line'] in unreachable_line_numbers:
            print(prop['sourceLocation']['line'], " ", prop['status'])

# # Generate the diff
# diff = difflib.unified_diff(output1.splitlines(), output2.splitlines(), lineterm='')

# # Write the diff to a file
# with open("output_diff.txt", "w") as file:
#     file.write('\n'.join(list(diff)))

# print("Difference between the outputs of the two commands has been stored in 'output_diff.txt' file.")
