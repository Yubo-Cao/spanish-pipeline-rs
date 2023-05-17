#let row = <ROW>
#let col = <COLUMN>
#let fontsize = <FONT_SIZE>

#set page(margin: 0.5in)
#let card = rect.with(
    inset: 8pt,
    stroke: (
        paint: rgb("#a3a3a3"),
        thickness: 0.5pt,
    ),
    width: 100%,
    height: 100%,
)
#let front_counter = counter("front")
#let front(content) = {
    front_counter.step()
    card[
        #place(
            top + left,
            text(
                fill: rgb("#a3a3a3"),
                front_counter.display()
            )
        )
        #align(center + horizon)[
            #text(
                fill: rgb("#0a0a0a"), 
                size: fontsize,
                content
            )
        ]
    ]
}
#let back(content) = {
    card[
        #align(center + horizon)[
            #text(
                fill: rgb("#737373"),
                size: fontsize,
                content
            )
        ]
    ]
}
#let card_layout = grid.with(
    columns: range(0, col).map(i => 1fr),
    rows: range(0, row).map(i => 1fr)
)

