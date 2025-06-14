// ignored
using Platform.Service;
using System.Collections.Generic;

namespace Platform.Social
{
// feature: type alias
using FriendId = User.Id;

public struct Friend
{
    public FriendId Id;

    // feature: List
    private List<Friend> Mutuals;
}
}
