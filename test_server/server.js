const Koa = require('koa');
const Router = require('koa-router');
const koaBody = require('koa-body');

const app = new Koa();
const router = new Router();

router.get('/test_get/:first_param/:second_param', async ctx => {
  console.log('==================== REQUEST GET ==================');
  console.log("Headers: ", ctx.headers);
  console.log("Query: ", ctx.query);
  console.log("QueryString: ", ctx.querystring);
  console.log("Path: ", ctx.path);
  console.log("Method: ", ctx.method);
  console.log("Params", ctx.params);
  ctx.body = 'Hello World';
  ctx.cookies.set("cookie-name", "cookie-value");
  ctx.cookies.set("cookie-name2", "cookie-value-2", {
    httpOnly: false
  });
});

router.post('/test_post', async ctx => {
  console.log('==================== REQUEST POST ==================');
  console.log("Headers: ", ctx.headers);
  console.log("Query: ", ctx.query);
  console.log("QueryString: ", ctx.querystring);
  console.log("Path: ", ctx.path);
  console.log("Method: ", ctx.method);
  console.log("Body: ", ctx.request.body);
  console.log("Cookie", ctx.cookies.get("cookie-name"));
  console.log("Cookie", ctx.cookies.get("cookie-name2"));
  ctx.body = 'Hello World';
});

app
  .use(koaBody())
  .use(router.routes())
  .use(router.allowedMethods());
app.listen(3000);