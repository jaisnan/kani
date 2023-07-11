import difflib
import subprocess
import json
import os

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
        if lines[-2] == ']':
            return '\n'.join(lines[:-1])  # Join lines excluding the last line
        elif lines[-4] == ']':
            return '\n'.join(lines[:-3])
    else:
        return ''  # If there is only one line or the string is empty, return an empty string

def get_rust_files(root_directory):
    # Traverse through the directory and its subdirectories
    rust_files = []
    for root, dirs, files in os.walk(directory):
        for filename in files:
            if filename.endswith(".rs"):
                file_path = os.path.join(root, filename)
                rust_files.append(file_path)

    return rust_files

files_without_unreachable_detected = []
files_with_discrepencies = []
files_with_matches = []

def run_commands(file_path):
    # Define the commands to run
    command1 = f"/home/ubuntu/kani/scripts/kani --enable-unstable --no-unwinding-checks --output-format=old {file_path} --cbmc-args --cover location --json-ui"
    command2 = f"/home/ubuntu/kani/scripts/kani --enable-unstable --no-unwinding-checks --output-format=old {file_path} --cbmc-args --json-ui"

    # Run the commands and get their outputs
    output1 = run_command(command1)
    output2 = run_command(command2)

    output1_cleaned = remove_last_line(remove_first_line(output1))
    output2_cleaned = remove_last_line(remove_first_line(output2))

    # Parse the outputs as JSON
    json_output1 = json.loads(output1_cleaned)
    json_output2 = json.loads(output2_cleaned)

    # Find reachability checks in output2
    found_reachability_check = []
    for item in json_output2:
        if 'result' in str(item):
            for prop in item['result']:
                if 'reachability_check' in str(prop):
                    # print(prop['sourceLocation']," ",prop['status'])
                    found_reachability_check.append((prop['sourceLocation'],prop['status']))

    unreachable_line_numbers = []
    for check in found_reachability_check:
        if check[1] == 'SUCCESS':
            unreachable_line_numbers.append(check[0]['line'])

    # print(file_path, "", unreachable_line_numbers)

    if len(unreachable_line_numbers) == 0:
        files_without_unreachable_detected.append(file_path)
        return

    # Find reachability checks in output1
    found_reachability_check_output1 = []
    found_match = False
    for item in json_output1:
        if 'goals' in str(item):
            for prop in item['goals']:
                if prop['sourceLocation']['line'] in unreachable_line_numbers:
                    print(file_path , " ", prop['sourceLocation']['line'], " ", prop['status'])
                    found_match = True

    if not found_match:
        files_with_discrepencies.append(file_path)
        # print(file_path, " ", "no unreachable line found")
    else:
        files_with_matches.append(file_path)


# Define the directory to parse for Rust files (including subdirectories)
directory = "/home/ubuntu/kani/tests/coverage/reach"
for file in get_rust_files(directory):
    run_commands(file)

print("\n")
print("Files without unreachable detected\n")
print(files_without_unreachable_detected, "\n")
print("files with discrepencies\n")
print(files_with_discrepencies, "\n")
print("Files that are okay")
print(files_with_matches, "\n")
