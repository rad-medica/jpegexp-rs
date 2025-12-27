import json
import os

nb_path = "tests/codec_comparison_result.ipynb"
if not os.path.exists(nb_path):
    print("Notebook file not found")
    exit(1)

with open(nb_path, 'r', encoding='utf-8') as f:
    nb = json.load(f)

print(f"Cells: {len(nb['cells'])}")
for i, cell in enumerate(nb['cells']):
    if cell['cell_type'] == 'code':
        source = "".join(cell['source'])[:50].replace("\n", " ")
        print(f"\nCell {i} [{source}...]:")
        for output in cell.get('outputs', []):
            if output['output_type'] == 'stream':
                text = "".join(output['text'])
                print(f"  Stream: {text.strip()}")
            elif output['output_type'] == 'error':
                print(f"  Error: {output['ename']} - {output['evalue']}")
            elif output['output_type'] == 'execute_result':
                print(f"  Result: {output.get('data', {}).get('text/plain', 'N/A')}")
            elif output['output_type'] == 'display_data':
                print("  Display: [Image/Data]")
