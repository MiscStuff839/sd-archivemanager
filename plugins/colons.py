import re
import extism

def add_colons(text: str) -> str:
    """
    For each line in `text`, find a leading '§...' index, count its dots,
    and prefix the line with ':'*(dots-1). Lines without a matching index
    are left unchanged.
    """
    def repl(match):
        index = match.group(1)               # e.g. "§1.1."
        rest  = match.group(2)               # the remainder of the line
        dots  = index.count('.')             # count how many '.' characters
        prefix = ':' * max(0, dots - 1)      # one colon per extra level
        return f"{prefix}{index}{rest}"

    pattern = re.compile(r'^(§[\d\.]+\.)(.*)$')
    lines = text.splitlines()
    out_lines = [pattern.sub(repl, line) for line in lines]
    return "\n".join(out_lines)

@extism.plugin_fn
def post():
    data = extism.input_json()
    print(data)
    data["content"] = add_colons(data["content"])
    extism.output_json(data)