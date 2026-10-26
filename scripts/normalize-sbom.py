import json
import sys
from pathlib import Path


def normalize(source, destination):
    document = json.loads(Path(source).read_text(encoding="utf-8"))
    component = document["metadata"]["component"]
    local_reference = component["bom-ref"]
    package_reference = f"pkg:cargo/{component['name']}@{component['version']}"

    def replace(value):
        if isinstance(value, dict):
            return {key: replace(item) for key, item in value.items()}
        if isinstance(value, list):
            return [replace(item) for item in value]
        if isinstance(value, str):
            if value.startswith(local_reference):
                return package_reference + value[len(local_reference):]
            if value.startswith(package_reference + "?download_url=file://."):
                return package_reference + value.split("?download_url=file://.", 1)[1]
        return value

    document = replace(document)
    if document.get("bomFormat") != "CycloneDX" or document.get("specVersion") != "1.5":
        raise ValueError("expected a generated CycloneDX 1.5 SBOM")
    if not document.get("components") or not document.get("dependencies"):
        raise ValueError("SBOM has no dependency inventory")
    Path(destination).write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    normalize(*sys.argv[1:])
