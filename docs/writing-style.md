# Writing style

The documentation in this repository follows ASD-STE100, Simplified Technical
English (STE). STE is the controlled-English standard from the AeroSpace and
Defence Industries Association of Europe. Aerospace teams use it so that a
maintenance manual reads the same way to every reader, in every language
background, with no second meaning.

We use STE for a plain reason. It removes the habits that make machine-written
prose easy to spot, and it makes a safety argument easier to check.

## The rules we apply

- Write short sentences. A sentence that gives an instruction has 20 words or
  fewer. A sentence that describes has 25 words or fewer.
- Write one instruction in one sentence.
- Use the active voice. Name the thing that does the action.
- Use the simple tenses: present, past, and future. Do not use the `-ing` form,
  except in a technical name.
- Use one word for one meaning. Use the same term for the same thing every time.
  For example, "the tool", "the plugin", "the operator", "the agent", "the
  model", "the core", and "the host" always mean the same thing.
- Keep articles. Write "the mint", not "mint".
- Do not use more than three nouns in a row.
- Start a section with its main point.
- Keep a paragraph to six sentences or fewer.
- Do not use slang, idiom, or a marketing word such as "seamless" or "leverage".

## Terms

| Term | Meaning |
|---|---|
| the agent | the ZeroClaw agent that runs the tools |
| the host | the ZeroClaw runtime that loads a component |
| the model | the large language model in the agent loop |
| the operator | the person who runs and configures the agent |
| the plugin | one Onca component and its manifest |
| the tool | the single callable the plugin exports |
| the core | the `onca-core` library |
| the component | the compiled `wasm32-wasip2` artifact |
