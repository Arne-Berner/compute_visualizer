# texture r8uint & co
Es gibt leider nur rgba8uint und co. Dadurch wird natürlich sehr viel mehr Speicher beansprucht als eigentlich notwendig.
Größere Sprünge heißt bei großen Daten auch, dass die GPU schneller ausgelastet ist und somit langsamer funktioniert.

Es funktioniert wohl einfach einen texture_2d zu nutzen, um an r8 ran zu kommen? Aber was genau ist das?    
