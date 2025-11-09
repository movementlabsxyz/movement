#!/bin/bash

# Exit on first error
set -e

# Check if input file parameter is provided
if [ $# -ne 1 ]; then
    echo "Usage: $0 <replay_logs_file>"
    echo "Example: $0 replay_logs"
    exit 1
fi

INPUT_FILE="$1"

# Check if input file exists
if [ ! -f "$INPUT_FILE" ]; then
    echo "Error: Input file '$INPUT_FILE' does not exist."
    exit 1
fi

# Get the directory of the input file
INPUT_DIR=$(dirname "$INPUT_FILE")

# Function to create output file path in the same directory as input
get_output_path() {
    echo "$INPUT_DIR/$1"
}

# Function to log file creation
log_creation() {
    echo "Creating file: $1"
}

echo "Processing log file: $INPUT_FILE"
echo "Output directory: $INPUT_DIR"
echo

# Step 1: Extract replay logs
CLEAN_LOGS=$(get_output_path "clean_logs")
log_creation "$CLEAN_LOGS"
grep "@R:" "$INPUT_FILE" | sed -e "s|.*@R:|@|g" > "$CLEAN_LOGS"

# Step 2: Extract submission results from replay logs
S_LOGS=$(get_output_path "S_logs")
log_creation "$S_LOGS"
grep "@S:" "$CLEAN_LOGS" | sed -e "s|@S:|@|g" > "$S_LOGS"

# Step 3: Split submission results into different scenarios
S_BS_LOGS=$(get_output_path "S_BS_logs")
log_creation "$S_BS_LOGS"
grep "@BS:" "$S_LOGS" | sed -E "s|@BS:(.*)|@\1|" > "$S_BS_LOGS"

S_BF_LOGS=$(get_output_path "S_BF_logs")
log_creation "$S_BF_LOGS"
grep "@BF:" "$S_LOGS" | sed -E "s|@BF:(.*)|@\1|" > "$S_BF_LOGS"

S_AF_LOGS=$(get_output_path "S_AF_logs")
log_creation "$S_AF_LOGS"
grep "@AF:" "$S_LOGS" | sed -E "s|@AF:(.*)|@\1|" > "$S_AF_LOGS"

S_MF_LOGS=$(get_output_path "S_MF_logs")
log_creation "$S_MF_LOGS"
grep "@MF:" "$S_LOGS" | sed -E "s|@MF:(.*)|@\1|" > "$S_MF_LOGS"

# Step 4: Extract execution results from replay logs
E_LOGS=$(get_output_path "E_logs")
log_creation "$E_LOGS"
grep "@E:" "$CLEAN_LOGS" | sed -e "s|@E:|@|g" > "$E_LOGS"

# Step 5: Split execution results into different scenarios
E_BS_LOGS=$(get_output_path "E_BS_logs")
log_creation "$E_BS_LOGS"
grep "@BS:" "$E_LOGS" | sed -E "s|@BS:(.*)|@\1|" > "$E_BS_LOGS"

E_BF_LOGS=$(get_output_path "E_BF_logs")
log_creation "$E_BF_LOGS"
grep "@BF:" "$E_LOGS" | sed -E "s|@BF:(.*)|@\1|" > "$E_BF_LOGS"

E_AF_LOGS=$(get_output_path "E_AF_logs")
log_creation "$E_AF_LOGS"
grep "@AF:" "$E_LOGS" | sed -E "s|@AF:(.*)|@\1|" > "$E_AF_LOGS"

E_MF_LOGS=$(get_output_path "E_MF_logs")
log_creation "$E_MF_LOGS"
grep "@MF:" "$E_LOGS" | sed -E "s|@MF:(.*)|@\1|" > "$E_MF_LOGS"

# Step 6: Filter for different outputs (events, changes) and errors
E_BS_DIFF_LOGS=$(get_output_path "E_BS_diff_logs")
log_creation "$E_BS_DIFF_LOGS"
grep -v "): ok" "$E_BS_LOGS" > "$E_BS_DIFF_LOGS"

E_BF_DIFF_LOGS=$(get_output_path "E_BF_diff_logs")
log_creation "$E_BF_DIFF_LOGS"
grep -v "same error" "$E_BF_LOGS" > "$E_BF_DIFF_LOGS"

# Step 7: Extract entry functions and abort functions for failed executions
E_AF_ENTRY=$(get_output_path "E_AF_entry")
log_creation "$E_AF_ENTRY"
grep "entry:" "$E_AF_LOGS" | sed -E "s|.*/entry:([_:a-zA-Z0-9]*)\).*|\1|" | sort | uniq > "$E_AF_ENTRY"

E_AF_ABORT=$(get_output_path "E_AF_abort")
log_creation "$E_AF_ABORT"
grep "Move abort in" "$E_AF_LOGS" | sed -E "s|.*Move abort in ([_:a-zA-Z0-9]*).*|\1|" | sort | uniq > "$E_AF_ABORT"

# Step 8: Clean up and prepare frequency files for failed entry and abort functions
E_AF_ENTRY_FREQ=$(get_output_path "E_AF_entry_freq")
E_AF_ABORT_FREQ=$(get_output_path "E_AF_abort_freq")

rm -f "$E_AF_ENTRY_FREQ"
rm -f "$E_AF_ABORT_FREQ"

# Step 9: Generate frequency counts for failed entry functions
log_creation "$E_AF_ENTRY_FREQ"
while IFS='' read -r LINE || [ -n "${LINE}" ]; do
    COUNT=$(grep -c "${LINE}" "$CLEAN_LOGS")
    echo "${LINE} ${COUNT}" >> "$E_AF_ENTRY_FREQ"
done < "$E_AF_ENTRY"

# Step 10: Generate frequency counts for abort functions
log_creation "$E_AF_ABORT_FREQ"
while IFS='' read -r LINE || [ -n "${LINE}" ]; do
    COUNT=$(grep -c "${LINE}" "$CLEAN_LOGS")
    echo "${LINE} ${COUNT}" >> "$E_AF_ABORT_FREQ"
done < "$E_AF_ABORT"

# Step 11: Generate statistics for transaction submissions
S_LOGS_STATS=$(get_output_path "S_logs_stats")
log_creation "$S_LOGS_STATS"

# Get number of submitted transactions
S_LOGS_COUNT=$(wc -l < "$S_LOGS")

# Get counts from individual files
S_BS_COUNT=$(wc -l < "$S_BS_LOGS")
S_BF_COUNT=$(wc -l < "$S_BF_LOGS")
S_AF_COUNT=$(wc -l < "$S_AF_LOGS")
S_MF_COUNT=$(wc -l < "$S_MF_LOGS")

# Calculate percentages
if [ "$S_LOGS_COUNT" -gt 0 ]; then
    S_BS_PERCENT=$(echo "scale=2; $S_BS_COUNT * 100 / $S_LOGS_COUNT" | bc)
    S_BF_PERCENT=$(echo "scale=2; $S_BF_COUNT * 100 / $S_LOGS_COUNT" | bc)
    S_AF_PERCENT=$(echo "scale=2; $S_AF_COUNT * 100 / $S_LOGS_COUNT" | bc)
    S_MF_PERCENT=$(echo "scale=2; $S_MF_COUNT * 100 / $S_LOGS_COUNT" | bc)
else
    S_BS_PERCENT=0.00
    S_BF_PERCENT=0.00
    S_AF_PERCENT=0.00
    S_MF_PERCENT=0.00
fi

# Write statistics to file
cat > "$S_LOGS_STATS" << EOF
Submission Statistics Report
============================

Base file: S_logs
Total submitted transactions: $S_LOGS_COUNT

Breakdown by category:
----------------------
Both succeeded: $S_BS_COUNT (${S_BS_PERCENT}%)
Both failed: $S_BF_COUNT (${S_BF_PERCENT}%)
Aptos submission failed: $S_AF_COUNT (${S_AF_PERCENT}%)
Movement submission failed: $S_MF_COUNT (${S_MF_PERCENT}%)

EOF

# Step 12: Generate statistics for transaction executions
E_LOGS_STATS=$(get_output_path "E_logs_stats")
log_creation "$E_LOGS_STATS"

# Get number of executed transactions
E_LOGS_COUNT=$(wc -l < "$E_LOGS")

# Get counts from individual files
E_BS_COUNT=$(wc -l < "$E_BS_LOGS")
E_BS_DIFF_COUNT=$(wc -l < "$E_BS_DIFF_LOGS")
E_BF_COUNT=$(wc -l < "$E_BF_LOGS")
E_BF_DIFF_COUNT=$(wc -l < "$E_BF_DIFF_LOGS")
E_AF_COUNT=$(wc -l < "$E_AF_LOGS")
E_MF_COUNT=$(wc -l < "$E_MF_LOGS")

# Calculate percentages
if [ "$E_LOGS_COUNT" -gt 0 ]; then
    E_BS_PERCENT=$(echo "scale=2; $E_BS_COUNT * 100 / $E_LOGS_COUNT" | bc)
    E_BF_PERCENT=$(echo "scale=2; $E_BF_COUNT * 100 / $E_LOGS_COUNT" | bc)
    E_AF_PERCENT=$(echo "scale=2; $E_AF_COUNT * 100 / $E_LOGS_COUNT" | bc)
    E_MF_PERCENT=$(echo "scale=2; $E_MF_COUNT * 100 / $E_LOGS_COUNT" | bc)
else
    E_BS_PERCENT=0.00
    E_BF_PERCENT=0.00
    E_AF_PERCENT=0.00
    E_MF_PERCENT=0.00
fi

# Write statistics to file
cat > "$E_LOGS_STATS" << EOF
Execution Statistics Report
===========================

Base file: E_logs
Total executed transactions: $E_LOGS_COUNT

Breakdown by category:
----------------------
Both succeeded: $E_BS_COUNT (${E_BS_PERCENT}%). Different outputs (events, changes): $E_BS_DIFF_COUNT.
Both failed: $E_BF_COUNT (${E_BF_PERCENT}%). Different errors: $E_BF_DIFF_COUNT
Aptos execution failed: $E_AF_COUNT (${E_AF_PERCENT}%)
Movement execution failed: $E_MF_COUNT (${E_MF_PERCENT}%)

EOF

echo
echo "Processing completed successfully!"
echo "All output files have been created in: $INPUT_DIR"
echo "Statistics summaries written to:"
echo "  - $S_LOGS_STATS"
echo "  - $E_LOGS_STATS"
