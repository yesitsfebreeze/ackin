-- The profile of this checkout. This repository is the base, not a
-- composition: it ships no cartridge, so there is nothing to name here. A
-- project that runs cartridges keeps its own .cartridge/init.lua listing the
-- cartridges installed under its cartridge root.
--
-- An empty profile still makes this directory a project, so `cartridge help`,
-- `cartridge settings` and `cartridge list` run here and answer for the base
-- alone. The tests compose their own throwaway profiles.
return {}
