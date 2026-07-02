# Test fixture image sources

Real underwater stills for manual/visual testing of the color engine
(golden-image regression tests use synthetic pixels, not these — see
`underwater-core/src/adjustments.rs`). Chosen for spread across water
conditions: clear blue (Red Sea), low-vis (cave), green/turbid
(alpine lake), temperate kelp forest (California), tropical macro/
strobe-lit (Philippines), dark murky wreck (Baltic Sea).

Deliberately **not** sourced from academic underwater-image-enhancement
benchmarks (UIEB, SQUID, RUIE, U45) — those are non-commercial-only or
have no license grant at all, which doesn't work for a commercial
open-core product. All fixtures below are Wikimedia Commons files with
an explicit, checked permissive license. Downsized to 1600px long edge
from source; license/attribution applies to the original.

| File | Source | Author | License |
|---|---|---|---|
| `coral_reef_pd.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Colorful_underwater_landscape_of_a_coral_reef.jpg) | Jim E Maragos, U.S. Fish and Wildlife Service | Public domain |
| `redsea_wreck_ccbysa.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Aft_view_Thistlegorm.jpg) | Wikimedia Commons contributor | CC BY-SA 3.0 |
| `redsea_cave_ccbysa.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:0610_Hurghada-Abu_Ramada_Cave-LaNaBueBa-DSCF4912.JPG) | Wikimedia Commons contributor | CC BY-SA 2.5 |
| `austrian_lake1_cc0.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:4822_Bad_Goisern_am_Hallst%C3%A4ttersee,_Grund_Hallst%C3%A4tter_See_03_2022-07-25.jpg) | Wikimedia Commons contributor | CC0 |
| `austrian_lake2_cc0.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:4822_Bad_Goisern_am_Hallst%C3%A4ttersee,_Wasserpflanzen_2022-07-25.jpg) | Wikimedia Commons contributor | CC0 |
| `kelp_batray_ccbysa.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Bat_Ray_in_kelp_forest,_San_Clemente_Island,_Channel_Islands,_California.jpg) | Wikimedia Commons contributor | CC BY-SA 2.5 |
| `kelp_rockfish_ccbysa.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Blue_Rockfish_in_kelp_forest.jpg) | Wikimedia Commons contributor | CC BY-SA 2.5 |
| `macro_ribboneel_ccbysa.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Anguila_list%C3%B3n_azul_(Rhinomuraena_quaesita),_Anilao,_Filipinas,_2023-08-23,_DD_59.jpg) | Wikimedia Commons contributor | CC BY-SA 4.0 |
| `wreck_applet_ccby.jpg` | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Vrak_%C3%84pplet_Undre_batterid%C3%A4ck.jpg) | Wikimedia Commons contributor | CC BY 4.0 |

CC BY-SA files require attribution + share-alike if redistributed —
this table satisfies attribution. These are development/test fixtures,
not shipped product assets.
