## Information Summary

Based on researching the topic, here's how to get user IDs for each platform:

### **Twitter (X) API**
- **Endpoint**: `GET https://api.twitter.com/2/users/by/username/{username}` [[3]](https://www.youtube.com/watch?v=GDbnarInwBQ)
- **Authentication**: Bearer Token (OAuth 2.0) [[4]](https://developer.twitter.com/apitools/api?endpoint=%2F2%2Ftweets%2F%7Bid%7D&method=get)
- **Response**: Returns user object with `id` field
- **Documentation**: [Twitter API v2 Users Lookup](https://developer.twitter.com/en/docs/twitter-api/users/lookup/api-reference/get-users-by-username-username)

### **Discord API**
- **Challenge**: Discord doesn't provide a public API to search users by username outside of shared servers [[7]](https://github.com/Rapptz/discord.py/discussions/6786)
- **Within a guild**: You can search members by username if the bot shares a server with them [[7]](https://github.com/Rapptz/discord.py/discussions/6786)
- **Endpoint**: Not directly available for global username lookup
- **Note**: Discord user IDs are Snowflake IDs (64-bit integers) [[8]](https://discord.com/developers/docs/reference)

### **Google/Gmail API**
- **API**: Google People API [[11]](https://groups.google.com/g/adwords-api/c/4ZiQ3JYLnK8)
- **Endpoint**: `GET https://people.googleapis.com/v1/people/{resourceName}` [[14]](https://developers.google.com/people/api/rest/v1/people/get)
- **Authentication**: OAuth 2.0 [[15]](https://developers.google.com/identity/protocols/oauth2)
- **Note**: Google doesn't provide direct email-to-user-ID lookup. You need OAuth flow to get authenticated user info
- **Gmail API**: `GET https://gmail.googleapis.com/gmail/v1/users/{userId}/profile` returns email address [[11]](https://stackoverflow.com/questions/54125066/how-can-i-get-the-userid-of-google-account-to-impact-with-their-api)
