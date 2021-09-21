import csv
import json

# read csv
csv_filename = "cgshop22.csv"
with open(csv_filename, newline='') as csvfile:
    r = csv.DictReader(csvfile)
    for row in r:
        inst_filename = row["path"]
        with open(inst_filename) as f:
            data = json.load(f)
            # print(len(data["preprocessed"]["dominations"]))
            # print(data["m"])
            print(len(list(filter(lambda e: e<300, data["preprocessed"]["degrees"]))))
