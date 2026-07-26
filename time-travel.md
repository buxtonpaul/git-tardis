# Commit time travel user interface.

## Terms and definitions

- Target commit  - This is the commit we are currently viewing when in time travel mode.
- Target candidates - This is the list of commits reachable from the target commit using the time travel navigation.
- File viewer pane - This is the pane used for looking at the contents of the file.
- Explorer pane - this is the sidebar pane used to view the current folder structure, commit lists, modified files etc.

## Operating behaviour
Directions on how the interface should behave in relation to the time travel mode.

### Time travel modes
We have the following time travel modes, these change the target commit as described below.
1. Commit based - Goes to the next/previous commit in the branches git history.
2. File based - Goes to the next/previous commit that affected the currently displayed file.
3. Function based - Goes to the next/previous commit that affected the current function.
4. Line based - Goes to the next/previous commit that affected the current line.

When we go back in time we need to be able to go forwards again!

### Expolorer pane

When displaying the list of commits moving the cursor up and down the list counts as changing the target commit, and should update the file viewer pane accordingly.
When the target commit changed due to other navigation events (e.g. using the navigation keys for line/function mode) the content displayed in the explorer pane should
update accordingly, e.g. if viewing the modified files list it should update to show the modified files for the new target commit.

### File viewer pane
Whilst navigating the file viewer pane we can invoke any of the time travel modes using the navigation keys and navigation mode. When this happens we change the target commit 
and the contents of the panes accordingly.
