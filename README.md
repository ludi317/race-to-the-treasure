# Race to the Treasure!

A Bevy implementation of the cooperative kids' board game [Race to the Treasure!](https://www.mindware.orientaltrading.com/race-to-the-treasure-peaceable-kingdom-cooperative-board-game-a2-GMC2.fltr?keyword=race%2bto%2bthe%2btreasure) by Peaceable Kingdom.

Beat the Ogre to the treasure chest at G7 by building a path from the west edge into the board, collecting 3 keys along the way.

![Screenshot](screenshot.png)

## Run

```
cargo run
```

## Controls

| Key / Click       | Action                                                            |
|-------------------|-------------------------------------------------------------------|
| `Space`           | Draw a card from the deck                                         |
| Left mouse button | Place the current path card on the hovered cell                   |
| `R`               | Rotate the current path card 90°                                  |
| `1`               | Spend a held Ogre Snack (removes the most recent ogre card)       |
| `Esc`             | Discard the current path card (e.g. nowhere legal to play it)     |
| `N`               | Start a new game                                                  |

## Rules

- **Setup**: 4 keys and 1 ogre snack are placed on random cells. The deck is 27 path cards (9 straight + 9 curve + 9 T-shape) + 10 ogre cards, shuffled together.
- **Turn**: draw one card.
  - **Ogre cards** land on the next empty cell of column G, top to bottom.
  - **Path cards** must be placed so at least one open end matches an open end on an adjacent card. The very first card goes on A0 and must have a West opening (connecting to the decorative start tile just off-board to its left).
- **Collecting**: covering a key or snack cell with a path card picks it up automatically. You need 3 keys to win.
- **Win**: cover the treasure with a path card while holding at least 3 keys.
- **Lose**: an ogre card lands on the treasure before you.
