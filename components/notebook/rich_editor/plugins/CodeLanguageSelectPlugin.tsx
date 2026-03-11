'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $getSelection,
  $isRangeSelection,
  COMMAND_PRIORITY_LOW,
  SELECTION_CHANGE_COMMAND,
} from 'lexical';
import { $isCodeNode } from '@lexical/code';
import { mergeRegister } from '@lexical/utils';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ScrollArea } from '@/components/ui/scroll-area';

import { SUPPORTED_CODE_LANGUAGES } from '../slash_commands/commands/codeBlock';

type PickerPosition = {
  top: number;
  left: number;
} | null;

export function CodeLanguageSelectPlugin() {
  const [editor] = useLexicalComposerContext();
  const [position, setPosition] = useState<PickerPosition>(null);
  const [language, setLanguage] = useState<string | undefined>(undefined);

  const languages = useMemo(() => SUPPORTED_CODE_LANGUAGES, []);

  useEffect(() => {
    const updatePositionAndLanguage = () => {
      let nextPosition: PickerPosition = null;
      let nextLanguage: string | undefined;

      editor.getEditorState().read(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          return;
        }

        const codeNode = selection.getNodes().find($isCodeNode);
        if (!codeNode) {
          return;
        }

        nextLanguage = codeNode.getLanguage() || languages[0];

        const codeElement = editor.getElementByKey(codeNode.getKey());
        const rect = codeElement?.getBoundingClientRect();

        if (rect) {
          const triggerWidth = 140;
          const inset = 16;
          nextPosition = {
            top: rect.top + window.scrollY + inset,
            left: rect.right + window.scrollX - triggerWidth - inset,
          };
        }
      });

      setPosition(nextPosition);
      setLanguage(nextLanguage);
    };

    return mergeRegister(
      editor.registerUpdateListener(() => {
        updatePositionAndLanguage();
      }),
      editor.registerCommand(
        SELECTION_CHANGE_COMMAND,
        () => {
          updatePositionAndLanguage();
          return false;
        },
        COMMAND_PRIORITY_LOW,
      ),
    );
  }, [editor, languages]);

  const handleChange = useCallback(
    (value: string) => {
      setLanguage(value);
      editor.update(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          return;
        }
        const codeNode = selection.getNodes().find($isCodeNode);
        if (codeNode) {
          codeNode.setLanguage(value);
        }
      });
    },
    [editor],
  );

  if (!position || !language) {
    return null;
  }

  return (
    <div
      className="fixed z-50"
      style={{
        top: position.top,
        left: position.left,
      }}
    >
      <Select value={language} onValueChange={handleChange}>
        <SelectTrigger className="h-9 w-36 rounded-md border bg-background text-sm text-foreground shadow-sm hover:bg-muted">
          <SelectValue placeholder="Language" />
        </SelectTrigger>
        <SelectContent className="border bg-background text-foreground">
          <ScrollArea className="max-h-60">
            {languages.map((lang) => (
              <SelectItem
                key={lang}
                value={lang}
                className="text-foreground data-[highlighted]:bg-muted data-[state=checked]:bg-muted"
              >
                {lang}
              </SelectItem>
            ))}
          </ScrollArea>
        </SelectContent>
      </Select>
    </div>
  );
}

export default CodeLanguageSelectPlugin;
