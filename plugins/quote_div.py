"""
wrap_toccolours.py

A script to wrap text enclosed in Unicode curly quotes (“ and ”) and/or standard double quotes (\")
with HTML <div class="toccolours" style="overflow:auto;">…</div> tags, supporting nested wrappers.

Usage:
    python wrap_toccolours.py < input.txt > output.txt
or:
    from wrap_toccolours import wrap_divs
    wrapped = wrap_divs(your_text)
"""

def wrap_divs(s,
              start_tag='<div class="toccolours" style="overflow:auto;">',
              end_tag='</div>',
              support_straight=True):
    """
    Wrap text delimited by curly quotes (“ and ”) and/or straight quotes (\")
    with nested divs. Each opening quote inserts a start tag, each closing quote
    inserts an end tag. Supports proper nesting and automatically closes
    any unmatched tags at the end.

    Args:
        s (str): Input text.
        start_tag (str): HTML to insert at each opening.
        end_tag (str): HTML to insert at each closing.
        support_straight (bool): Whether to treat standard double quotes as wrappers.

    Returns:
        str: Text with <div> wrappers.
    """
    result = []
    depth = 0
    straight_open = True  # toggles for straight quotes
    for ch in s:
        # Handle straight double-quote
        if support_straight and ch == '"':
            if straight_open:
                result.append(start_tag)
                depth += 1
            else:
                if depth > 0:
                    result.append(end_tag)
                    depth -= 1
                else:
                    # no matching opener
                    result.append(ch)
            straight_open = not straight_open
        # Handle curly opening quote “ (U+201C)
        elif ch == '\u201C':
            result.append(start_tag)
            depth += 1
        # Handle curly closing quote ” (U+201D)
        elif ch == '\u201D':
            if depth > 0:
                result.append(end_tag)
                depth -= 1
            else:
                result.append(ch)
        else:
            result.append(ch)
    # Close any unmatched divs
    result.extend(end_tag for _ in range(depth))
    return ''.join(result)


def main():
    import sys
    text = sys.stdin.read()
    wrapped = wrap_divs(text)
    sys.stdout.write(wrapped)


if __name__ == '__main__':
    main()
